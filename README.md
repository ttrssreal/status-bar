# Status Bar for dwm

It sets the display's root window-name to some status-bar text, dwm then renders that at the top right of the screen.

It does that once every second, in time with the clock. It also spins up another thread listening for dbus method calls to `Update` on `org.user.StatusBar` which it then also updates the bar. Now any other app (such as one controlling volume) can update the bar when they change some system state. yay.

### Build
Install font-awesome.\
`cargo build [--release]`