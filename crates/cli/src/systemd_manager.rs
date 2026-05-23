use zbus::{proxy, Result};
use zbus::zvariant::OwnedObjectPath;

#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
pub trait SystemdManager {
    fn reload(&self) -> Result<()>;

    fn restart_unit(
        &self,
        name: &str,
        mode: &str,
    ) -> Result<OwnedObjectPath>;

    fn stop_unit(
        &self,
        name: &str,
        mode: &str,
    ) -> Result<OwnedObjectPath>;

    fn enable_unit_files(
        &self,
        files: Vec<&str>,
        runtime: bool,
        force: bool,
    ) -> Result<(bool, Vec<(String, String, String)>)>;

    fn disable_unit_files(
        &self,
        files: Vec<&str>,
        runtime: bool,
    ) -> Result<Vec<(String, String, String)>>;

    fn mask_unit_files(
        &self,
        files: Vec<&str>,
        runtime: bool,
        force: bool,
    ) -> Result<Vec<(String, String, String)>>;

    fn unmask_unit_files(
        &self,
        files: Vec<&str>,
        runtime: bool,
    ) -> Result<Vec<(String, String, String)>>;
}
