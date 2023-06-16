use std::sync::Arc;
use std::time::Duration;
use x11rb;
use x11rb::rust_connection::{RustConnection, ConnectionError};
use x11rb::wrapper::ConnectionExt;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{PropMode, AtomEnum, Window};
use anyhow::{bail, Context, Ok};
use chrono::Local;
use zbus::{self, MessageType, MessageStream};
use zbus::export::futures_util::TryStreamExt;

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

pub fn get_time_component() -> String {
	return Local::now()
		.format("%a %d %b %H:%M:%S")
		.to_string()
		.clone();
}

pub fn get_vol_component() -> anyhow::Result<String> {
	let result = std::process::Command::new("bash")
		.arg("-c")
		.arg("amixer -D pipewire sget Master 			\
				| tail -n 1								\
				| grep --colour=never -o '[0-9]\\+%'	\
				| tr -d '\\n'")
		.output()?;
	if !result.status.success() {
		bail!("Couldn't fetch audio level with 'amixer' (tail and grep).");
	}
	return Ok(std::str::from_utf8(&result.stdout[..])?.to_string());
}

pub fn get_battery_component() -> anyhow::Result<String> {
	let percentage = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity")?.trim().to_string();
	return Ok(percentage);
}

pub fn get_status_bar() -> anyhow::Result<String> {
	let time = get_time_component();
	let vol = get_vol_component()?;
	let batt = get_battery_component()?;
	// battery and sound icons from font-awesome
	return Ok(format!("  {}% |  {} | {} ", batt, vol, time));
}

pub async fn handle_dbus_calls(conn: &RustConnection, root: Window, dbus_stream: &mut MessageStream) -> anyhow::Result<()> {
	while let Some(msg) = dbus_stream.try_next().await? {
		let msg_header = msg.header()?;
		if msg_header.message_type()? != MessageType::MethodCall { continue; }
		if msg.member().context("No member name on dbus-call")?.as_str() != "Update" { continue;}
		let win_name = get_status_bar()?;
		set_status_bar(&conn, root, &win_name)?;
	}
	Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let conn = Arc::new(x11rb::connect(None)?.0);
	
	let roots = &conn.setup().roots;
	if roots.len() < 1 {
		bail!("No root window");
	}
	let root: u32 = roots[0].root;

	let connection = zbus::Connection::session()
        .await?;
    connection.request_name("org.user.StatusBar")
        .await?;

	let mut stream = zbus::MessageStream::from(&connection);

	let conn_dbus_thread = conn.clone();

	tokio::spawn(async move {
		if let Err(err) = handle_dbus_calls(&conn_dbus_thread, root,&mut stream).await {
			eprintln!("{err}");
			std::process::exit(1);
		}
	});
	
	loop {
		let win_name = get_status_bar()?;
		set_status_bar(&conn, root, &win_name)?;

		tokio::time::sleep(Duration::from_secs(1)).await;
	}
}