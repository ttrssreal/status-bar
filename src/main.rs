mod status_bar;

use status_bar::StatusBar;
use tokio::task::JoinSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::process;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::process::Output;
use regex::Regex;
use x11rb;
use x11rb::rust_connection::{RustConnection, ConnectionError};
use x11rb::wrapper::ConnectionExt;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{PropMode, AtomEnum, Window};
use anyhow::{bail, Ok};
use std::env;
use chrono::Local;

fn set_status_bar(
		conn: &RustConnection,
		window: Window, 
		content: &str
	) -> Result<(), ConnectionError> {
	conn.change_property8(
		PropMode::REPLACE,
		window,
		AtomEnum::WM_NAME,
		AtomEnum::STRING,
		content.as_bytes()
	)?;
	conn.flush()?;
	Result::<(), ConnectionError>::Ok(())
}

pub fn start_time_component(
        status_bar: Arc<StatusBar>,
        joinset: &mut JoinSet<()>,
        update_handle: Sender<()>)
{
	joinset.spawn(async move {
        loop {
            {
                let mut time = status_bar.time.lock().unwrap();
                time.clear();
                time.push_str(&Local::now().format("%a %d %b %H:%M:%S").to_string());
                update_handle.send(()).expect("Can't trigger update!");
            }
            tokio::time::sleep(Duration::from_millis(status_bar.time_update_period)).await;
        }
    });
}

pub fn start_volume_component(
    status_bar: Arc<StatusBar>,
    joinset: &mut JoinSet<()>,
    update_handle: Sender<()>)
{
	joinset.spawn(async move {
        let audio_level_re = Regex::new(r"\[(\d+%)\]").unwrap();
        let err_msg = "failed to run 'amixer', is it installed?";
        loop {
            let amixer_out = process::Command::new("amixer")
                .arg("get")
                .arg("Master")
                .output()
                .await;
            let res = match amixer_out {
                Result::Ok(Output { status, stdout, .. }) => {
                    if status.success() {
                        stdout
                    } else {
                        println!("{err_msg}");
                        continue;
                    }
                },
                Err(_) => panic!("{}", err_msg)
            };
            let audio_info = String::from_utf8_lossy(res.as_slice());
            let audio_level_line = match audio_info
                .lines()
                .find(|line| { line.contains("Front Left:") }) {
                    Some(s) => s,
                    _ => {
                        eprintln!("Bad 'amixer' output");
                        continue;
                    }

            };
            let audio_level = &(match audio_level_re
                .captures(audio_level_line) {
                    Some(s) => s,
                    _ => {
                        eprintln!("Bad 'amixer' output");
                        continue;
                    }
                })[1];
            let value = audio_level[0..audio_level.len() - 1].parse::<u32>()
                .unwrap();
            let icon = match value {
                0 => "\u{f026}",
                1..=75 => "\u{f027}",
                76..=100 => "\u{f028}",
                _ => "\u{f06d}",
            };
            {
                let mut volume = status_bar.volume.lock().unwrap();
                volume.clear();
                volume.push_str(&format!("{icon} {audio_level}"));
                update_handle.send(()).expect("Can't trigger update!");
            }
            tokio::time::sleep(Duration::from_millis(status_bar.volume_update_period)).await;
        }
    });
}

pub fn start_battery_component(
    status_bar: Arc<StatusBar>,
    joinset: &mut JoinSet<()>,
    update_handle: Sender<()>)
{
	joinset.spawn(async move {
        loop {
            let percentage = std::fs::read_to_string(env::var("BATTERY_CAPACITY_DEVICE")
                                                     .expect("BATTERY_CAPACITY_DEVICE"))
                .expect("Can't read battery level")
	        	.trim()
	        	.parse::<i32>()
                .unwrap();
	        let battery_icon = match percentage {
	        	0..=10 => "\u{f06d}",
	        	11..=15 => "\u{f244}",
	        	16..=50 => "\u{f243}",
	        	51..=75 => "\u{f242}",
	        	76..=99 => "\u{f241}",
	        	100 => "\u{f240}",
	        	_ => "\u{f06d}",
	        };
            let charging_status = std::fs::read_to_string(env::var("BATTERY_STATUS_DEVICE")
                                                          .expect("BATTERY_STATUS_DEVICE"))
                .expect("Can't read battery status");
            let charging_icon = match charging_status.trim() {
                "Charging" => "\u{f0e7} ",
                _ => ""
            };
            {
                let mut battery = status_bar.battery.lock().unwrap();
                battery.clear();
                battery.push_str(&format!("{charging_icon}{battery_icon} {percentage}%"));
                update_handle.send(()).expect("Can't trigger update!");
            }
            tokio::time::sleep(Duration::from_millis(status_bar.battery_update_period)).await;
        }
    });
}

pub fn start_wifi_component(
    status_bar: Arc<StatusBar>,
    joinset: &mut JoinSet<()>,
    update_handle: Sender<()>)
{
	joinset.spawn(async move {
        let err_msg = "failed to run 'nmcli', is it installed?";
        loop {
            let nmcli_out = process::Command::new("nmcli")
                .arg("-g").arg("general.connection")
                .arg("device")
                .arg("show")
                .arg(env::var("WIFI_DEVICE_NAME").expect("WIFI_DEVICE_NAME"))
                .output()
                .await;
            let res = match nmcli_out {
                Result::Ok(Output { status, stdout, stderr }) => {
                    if status.success() {
                        stdout
                    } else {
                        panic!("{err_msg}: {stderr:?}")
                    }
                },
                Err(_) => panic!("{}", err_msg)
            };
            let ssid = String::from_utf8_lossy(res.as_slice());
            let ssid = ssid.trim();
            let connected = ssid.len() != 0;
	        let icon = if connected { "\u{f1eb} " } else { "\u{f05e}" };
            {
                let mut wifi = status_bar.wifi.lock().unwrap();
                wifi.clear();
                wifi.push_str(&format!("{icon}{ssid}"));
                update_handle.send(()).expect("Can't trigger update!");
            }
            tokio::time::sleep(Duration::from_millis(status_bar.wifi_update_period)).await;
        }
    });
}

pub fn start_update_status_bar(
    status_bar: Arc<StatusBar>,
    joinset: &mut JoinSet<()>,
    update_requests: Receiver<()>
) -> anyhow::Result<()> {
    // Connect to X11 server
	let (x11_conn, _) = x11rb::connect(None)?;
	let screens = &x11_conn.setup().roots;
	if screens.len() < 1 {
		bail!("No root window");
	}
	let root_window = screens[0].root;
    println!("X11 connection established!");

    joinset.spawn(async move {
        while let Result::Ok(_) = update_requests.recv() {
            let status_bar_string = status_bar.render();
            let _ = set_status_bar(&x11_conn, root_window, &status_bar_string);
        } 
    });

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let status_bar = Arc::new(StatusBar::new());
    let (update_handle, update_requests) = channel();
    let mut joinset = JoinSet::new();

    start_update_status_bar(status_bar.clone(), &mut joinset, update_requests)?;
    start_time_component(status_bar.clone(), &mut joinset, update_handle.clone());
    start_volume_component(status_bar.clone(), &mut joinset, update_handle.clone());
    start_battery_component(status_bar.clone(), &mut joinset, update_handle.clone());
    start_wifi_component(status_bar.clone(), &mut joinset, update_handle.clone());

    while let Some(_) = joinset.join_next().await {}

    Ok(())
}
