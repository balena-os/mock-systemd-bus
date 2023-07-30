use log::{error, info, Level};
use serde::{Deserialize, Serialize};
use std::env;
use std::fmt;
use zbus::{
    dbus_interface,
    zvariant::{ObjectPath, Type},
    ConnectionBuilder, ObjectServer,
};

/// The active state of a systemd unit
///
/// Valid values according to https://www.freedesktop.org/wiki/Software/systemd/dbus/
/// are: active, reloading, inactive, failed, activating, deactivating
#[derive(Deserialize, Serialize, Type, PartialEq, Debug, Clone)]
enum UnitActiveState {
    Active,
    Reloading,
    Inactive,
    Failed,
    Activating,
    Deactivating,
}

impl fmt::Display for UnitActiveState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnitActiveState::Active => write!(f, "active"),
            UnitActiveState::Reloading => write!(f, "reloading"),
            UnitActiveState::Inactive => write!(f, "inactive"),
            UnitActiveState::Failed => write!(f, "failed"),
            UnitActiveState::Activating => write!(f, "activating"),
            UnitActiveState::Deactivating => write!(f, "deactivating"),
        }
    }
}

#[derive(Deserialize, Serialize, Type, PartialEq, Debug, Clone)]
struct Unit {
    name: String,
    active_state: UnitActiveState,
}

#[dbus_interface(name = "org.freedesktop.systemd1.Unit")]
impl Unit {
    #[dbus_interface(property)]
    async fn active_state(&self) -> String {
        self.active_state.to_string()
    }

    #[dbus_interface(property)]
    async fn part_of(&self) -> Vec<String> {
        vec![]
    }
}

struct ServiceManager;

#[derive(zbus::DBusError, Debug)]
enum Error {
    #[dbus_error(zbus_error)]
    ZBus(zbus::Error),
    InvalidPath(String),
    UnitAlreadyExists,
}

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

fn object_path(path: &'static str) -> Result<ObjectPath<'static>, Error> {
    ObjectPath::try_from(path).map_err(|e| Error::InvalidPath(e.to_string()))
}

fn unit_path(name: &str) -> Result<ObjectPath<'static>, Error> {
    let name = name.to_lowercase().replace('.', "_");
    let path = format!("/org/freedesktop/systemd1/unit/{}", name);
    ObjectPath::try_from(path).map_err(|e| Error::InvalidPath(e.to_string()))
}

#[dbus_interface(name = "org.freedesktop.systemd1.Manager")]
impl ServiceManager {
    /// Create a fake unit. Obviously this is not part of the
    /// standard systemd API but it allows us to use the bus to
    /// create a unit for testing purposes
    async fn mock_add_unit(
        &self,
        #[zbus(object_server)] server: &ObjectServer,
        name: &str,
    ) -> Result<ObjectPath, Error> {
        let interface = Unit {
            name: name.to_string(),
            active_state: UnitActiveState::Inactive,
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
        let success = server.remove::<Unit, _>(path).await?;
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
        server.interface::<_, Unit>(path.clone()).await?;

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
        let interface = server.interface::<_, Unit>(path.clone()).await?;
        let mut interface = interface.get_mut().await;
        interface.active_state = UnitActiveState::Active;

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
        let interface = server.interface::<_, Unit>(path.clone()).await?;
        let mut interface = interface.get_mut().await;
        interface.active_state = UnitActiveState::Inactive;

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
        let interface = server.interface::<_, Unit>(path.clone()).await?;
        let mut interface = interface.get_mut().await;

        interface.active_state = UnitActiveState::Active;

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

    // setup the service manager
    let service = ConnectionBuilder::system()?
        .name("org.freedesktop.systemd1")?
        .serve_at("/org/freedesktop/systemd1", ServiceManager)?
        .build()
        .await?;

    // Add default units from command line arguments
    // if any. We need to do this so they are available as soon
    // as tests need it
    let args: Vec<String> = env::args().collect();
    for arg in args.iter().skip(1) {
        service
            .call_method(
                Some("org.freedesktop.systemd1"),
                "/org/freedesktop/systemd1",
                Some("org.freedesktop.systemd1.Manager"),
                "MockAddUnit",
                &(arg,),
            )
            .await?;
    }

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
