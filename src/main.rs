use log::{info, Level};
use std::fmt;
use zbus::{dbus_interface, ConnectionBuilder};

#[derive(Default)]
enum PowerState {
    #[default]
    Ready,
    Rebooting,
    Off,
}

impl fmt::Display for PowerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PowerState::Ready => write!(f, "ready"),
            PowerState::Rebooting => write!(f, "rebooting"),
            PowerState::Off => write!(f, "off"),
        }
    }
}

#[derive(Default)]
struct LoginManager {
    state: PowerState,
}

#[dbus_interface(name = "org.freedesktop.login1.Manager")]
impl LoginManager {
    async fn reboot(&mut self, _interactive: bool) {
        self.state = PowerState::Rebooting;
        info!("system is rebooting");
    }

    async fn power_off(&mut self, _interactive: bool) {
        self.state = PowerState::Off;
        info!("system is off");
    }

    /// Not a real login manager method, but useful for testing
    async fn mock_reset(&mut self) {
        self.state = PowerState::Ready;
        info!("system is ready");
    }

    /// Not a real login manager property, but useful for testing
    #[dbus_interface(property)]
    async fn mock_state(&self) -> String {
        self.state.to_string()
    }
}

// Although we use `async-std` here, you can use any async runtime of choice.
#[async_std::main]
async fn main() -> zbus::Result<()> {
    stderrlog::new()
        .module(module_path!())
        .verbosity(Level::Warn)
        .init()
        .unwrap();

    // setup login manager
    let _login = ConnectionBuilder::system()?
        .name("org.freedesktop.login1")?
        .serve_at("/org/freedesktop/login1", LoginManager::default())?
        .build()
        .await?;

    info!("started!");

    loop {
        // do something else, wait forever or timeout here:
        // handling D-Bus messages is done in the background
        std::future::pending::<()>().await;
    }
}
