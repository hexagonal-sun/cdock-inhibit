use anyhow::Result;
use std::os::fd::{AsFd, AsRawFd};
use udev::Device;
use zbus::{blocking::Connection, proxy};

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait LoginManager {
    /// Inhibit method
    fn inhibit(
        &self,
        what: &str,
        who: &str,
        why: &str,
        mode: &str,
    ) -> zbus::Result<zbus::zvariant::OwnedFd>;
}

struct InhibitLock {
    _fd: zbus::zvariant::OwnedFd,
}

impl InhibitLock {
    fn take() -> Result<Self> {
        let conn = Connection::system()?;
        let lmgr = LoginManagerProxyBlocking::new(&conn)?;

        let fd = lmgr.inhibit(
            "sleep:handle-lid-switch",
            "cdock-inhibit",
            "Docked via USB-C",
            "block",
        )?;

        Ok(Self { _fd: fd })
    }
}

fn is_device_dock(dev: &Device) -> bool {
    dev.property_value("ID_VENDOR_ID")
        .map(|v| v == "413c")
        .unwrap_or(false)
        && dev
            .property_value("ID_MODEL_ID")
            .map(|v| v == "b06f")
            .unwrap_or(false)
}

fn main() -> Result<()> {
    let monitor = udev::MonitorBuilder::new()?
        .match_subsystem_devtype("usb", "usb_device")?
        .listen()?;

    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_subsystem("usb")?;

    let mut devices = enumerator.scan_devices()?;

    let mut _inhibit_lock = match devices.find(is_device_dock) {
        Some(dev) => Some((InhibitLock::take()?, dev.devpath().to_owned())),
        None => None,
    };

    let fd = monitor.as_fd().as_raw_fd();

    // By default libudev makes the monitor socket non-blocking. Remove the
    // O_NONBLOCK flag so we can simply block when waiting for events (this
    // seems to work!)
    unsafe {
        libc::fcntl(
            fd,
            libc::F_SETFL,
            libc::fcntl(fd, libc::F_GETFD) & !libc::O_NONBLOCK,
        )
    };

    for event in monitor.iter() {
        if !match _inhibit_lock.as_ref() {
            Some((_, devpath)) => event.devpath() == *devpath,
            None => is_device_dock(&event.device()),
        } {
            continue;
        }

        match event.event_type() {
            udev::EventType::Add => {
                _inhibit_lock = Some((InhibitLock::take()?, event.devpath().to_owned()))
            }
            udev::EventType::Remove => _inhibit_lock = None,
            _ => {}
        }
    }

    Ok(())
}
