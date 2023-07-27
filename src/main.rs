use log::{error, info, Level};
use serde::{Deserialize, Serialize};
use std::fmt;
use zbus::{dbus_interface, ConnectionBuilder, ObjectServer};
use zvariant::{ObjectPath, Type};

/// The active state of a systemd unit
///
/// Valid values according to https://www.freedesktop.org/wiki/Software/systemd/dbus/
/// are: active, reloading, inactive, failed, activating, deactivating
#[derive(Deserialize, Serialize, Type, PartialEq, Debug, Clone)]
enum SystemdUnitActiveState {
    Active,
    Reloading,
    Inactive,
    Failed,
    Activating,
    Deactivating,
}

impl fmt::Display for SystemdUnitActiveState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemdUnitActiveState::Active => write!(f, "active"),
            SystemdUnitActiveState::Reloading => write!(f, "reloading"),
            SystemdUnitActiveState::Inactive => write!(f, "inactive"),
            SystemdUnitActiveState::Failed => write!(f, "failed"),
            SystemdUnitActiveState::Activating => write!(f, "activating"),
            SystemdUnitActiveState::Deactivating => write!(f, "deactivating"),
        }
    }
}

#[derive(Deserialize, Serialize, Type, PartialEq, Debug, Clone)]
struct SystemdUnit {
    name: String,
    active_state: SystemdUnitActiveState,
}

#[dbus_interface(name = "org.freedesktop.systemd1.Unit")]
impl SystemdUnit {
    #[dbus_interface(property)]
    async fn active_state(&self) -> String {
        self.active_state.to_string()
    }

    #[dbus_interface(property)]
    async fn part_of(&self) -> Vec<String> {
        vec![]
    }
}

struct SystemdManager;

#[derive(zbus::DBusError, Debug)]
enum Error {
    #[dbus_error(zbus_error)]
    ZBus(zbus::Error),
    InvalidPath(String),
    UnitAlreadyExists,
}

#[derive(Default)]
enum SystemState {
    #[default]
    Ready,
    Rebooting,
    Off,
}

impl fmt::Display for SystemState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SystemState::Ready => write!(f, "ready"),
            SystemState::Rebooting => write!(f, "rebooting"),
            SystemState::Off => write!(f, "off"),
        }
    }
}

#[derive(Default)]
struct LoginManager {
    state: SystemState,
}

#[dbus_interface(name = "org.freedesktop.login1.Manager")]
impl LoginManager {
    async fn reboot(&mut self, _interactive: bool) {
        self.state = SystemState::Rebooting;
        info!("system is rebooting");
    }

    async fn power_off(&mut self, _interactive: bool) {
        self.state = SystemState::Off;
        info!("system is off");
    }

    /// Not a real login manager method, but useful for testing
    async fn mock_reset(&mut self) {
        self.state = SystemState::Ready;
        info!("system is ready");
    }

    /// Not a real login manager property, but useful for testing
    #[dbus_interface(property)]
    async fn mock_state(&self) -> String {
        self.state.to_string()
    }
}

fn object_path(path: &'static str) -> Result<ObjectPath<'static>, Error> {
    ObjectPath::try_from(path).map_err(|e| Error::InvalidPath(e.to_string()))
}

fn unit_path(name: &str) -> Result<ObjectPath<'static>, Error> {
    let name = name.to_lowercase().replace('.', "_");
    let path = format!("/org/freedesktop/systemd1/unit/{}", name);
    ObjectPath::try_from(path).map_err(|e| Error::InvalidPath(e.to_string()))
}

#[dbus_interface(name = "org.freedesktop.systemd1.Manager")]
impl SystemdManager {
    /// Create a fake unit. Obviously this is not part of the
    /// standard systemd API but it allows us to use the bus to
    /// create a unit for testing purposes
    async fn mock_add_unit(
        &self,
        #[zbus(object_server)] server: &ObjectServer,
        name: &str,
    ) -> Result<ObjectPath, Error> {
        let interface = SystemdUnit {
            name: name.to_string(),
            active_state: SystemdUnitActiveState::Inactive,
        };

        let path = unit_path(name)?;
        if !server.at(path.clone(), interface).await? {
            error!("unit already exists: {}", name);
            return Err(Error::UnitAlreadyExists);
        }
        info!("created unit '{}' with path '{}'", name, path);

        Ok(path)
    }

    /// Remove a fake unit. Obviously this is not part of the
    /// standard systemd API but it allows us to use the bus to
    /// manipulate a unit for testing purposes
    async fn mock_del_unit(
        &self,
        #[zbus(object_server)] server: &ObjectServer,
        name: &str,
    ) -> Result<bool, Error> {
        let path = unit_path(name)?;
        let success = server.remove::<SystemdUnit, _>(path).await?;
        if success {
            info!("removed unit '{}'", name);
        }

        Ok(success)
    }

    async fn get_unit(
        &self,
        #[zbus(object_server)] server: &ObjectServer,
        name: &str,
    ) -> Result<ObjectPath, Error> {
        let path = unit_path(name)?;

        // Try to get the interface. This will fail if the
        // interface does not exist
        server.interface::<_, SystemdUnit>(path.clone()).await?;

        Ok(path)
    }

    async fn start_unit(
        &self,
        #[zbus(object_server)] server: &ObjectServer,
        name: &str,
        _mode: &str,
    ) -> Result<ObjectPath, Error> {
        let path = unit_path(name)?;

        // Try to get the interface. This will fail if the
        // interface does not exist
        let interface = server.interface::<_, SystemdUnit>(path.clone()).await?;
        let mut interface = interface.get_mut().await;
        interface.active_state = SystemdUnitActiveState::Active;

        info!("started unit '{}'", name);

        // Return a job number to be consistent with systemd
        // output. Perhaps we'll implement jobs with the JobRemoved
        // signal at some point
        let job = object_path("/org/freedesktop/systemd1/job/1")?;

        Ok(job)
    }

    async fn stop_unit(
        &self,
        #[zbus(object_server)] server: &ObjectServer,
        name: &str,
        _mode: &str,
    ) -> Result<ObjectPath, Error> {
        let path = unit_path(name)?;

        // Try to get the interface. This will fail if the
        // interface does not exist
        let interface = server.interface::<_, SystemdUnit>(path.clone()).await?;
        let mut interface = interface.get_mut().await;
        interface.active_state = SystemdUnitActiveState::Inactive;

        info!("stopped unit '{}'", name);

        let job = object_path("/org/freedesktop/systemd1/job/1")?;

        Ok(job)
    }

    async fn restart_unit(
        &self,
        #[zbus(object_server)] server: &ObjectServer,
        name: &str,
        _mode: &str,
    ) -> Result<ObjectPath, Error> {
        let path = unit_path(name)?;

        // Try to get the interface. This will fail if the
        // interface does not exist
        let interface = server.interface::<_, SystemdUnit>(path.clone()).await?;
        let mut interface = interface.get_mut().await;

        interface.active_state = SystemdUnitActiveState::Active;

        info!("restarted unit '{}'", name);

        let job = object_path("/org/freedesktop/systemd1/job/1")?;

        Ok(job)
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

    // setup the server
    let _systemd = ConnectionBuilder::system()?
        .name("org.freedesktop.systemd1")?
        .serve_at("/org/freedesktop/systemd1", SystemdManager)?
        .build()
        .await?;

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
