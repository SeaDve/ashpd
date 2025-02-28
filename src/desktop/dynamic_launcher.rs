//! # Examples
//!
//! ```rust,no_run
//! use ashpd::{
//!     desktop::{
//!         dynamic_launcher::{DynamicLauncherProxy, LauncherType, PrepareInstallOptions},
//!         Icon,
//!     },
//!     WindowIdentifier,
//! };
//!
//! async fn run() -> ashpd::Result<()> {
//!     let proxy = DynamicLauncherProxy::new().await?;
//!     let (name, token) = proxy
//!         .prepare_install(
//!             &WindowIdentifier::default(),
//!             "My App",
//!             Icon::with_names(&["dialog-symbolic"]),
//!             PrepareInstallOptions::default(),
//!         )
//!         .await?
//!         .response()?;
//!
//!     // Name and Icon will be overwritten from what we provided above
//!     // Exec will be overridden to call `flatpak run our-app` if the application is sandboxed
//!     let desktop_entry = r#"
//!         [Desktop Entry]
//!         Comment=My Web App
//!         Type=Application
//!     "#;
//!     proxy
//!         .install(&token, "some_file.desktop", desktop_entry)
//!         .await?;
//!
//!     proxy.uninstall("some_file.desktop").await?;
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;

use enumflags2::{bitflags, BitFlags};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use zbus::zvariant::{self, SerializeDict, Type};

use super::{HandleToken, Icon, Request};
use crate::{proxy::Proxy, Error, WindowIdentifier};

#[bitflags]
#[derive(Default, Serialize_repr, Deserialize_repr, PartialEq, Eq, Debug, Copy, Clone, Type)]
#[repr(u32)]
#[doc(alias = "XdpLauncherType")]
/// The type of the launcher.
pub enum LauncherType {
    #[doc(alias = "XDP_LAUNCHER_APPLICATION")]
    #[default]
    /// A launcher that represents an application
    Application,
    #[doc(alias = "XDP_LAUNCHER_WEBAPP")]
    /// A launcher that represents a web application
    WebApplication,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Type)]
#[zvariant(signature = "s")]
#[serde(rename_all = "lowercase")]
/// The icon format.
pub enum IconType {
    /// PNG.
    Png,
    /// JPEG.
    Jpeg,
    /// SVG.
    Svg,
}

#[derive(Deserialize, Type)]
#[zvariant(signature = "(vsu)")]
/// The icon of the launcher.
pub struct LauncherIcon(Icon, IconType, u32);

impl LauncherIcon {
    /// The actual icon.
    pub fn icon(&self) -> &Icon {
        &self.0
    }

    /// The icon type.
    pub fn type_(&self) -> IconType {
        self.1
    }

    /// The icon size.
    pub fn size(&self) -> u32 {
        self.2
    }
}

#[derive(Debug, Default, SerializeDict, Type)]
#[zvariant(signature = "dict")]
/// Options to pass to [`DynamicLauncherProxy::prepare_install`]
pub struct PrepareInstallOptions {
    handle_token: HandleToken,
    modal: Option<bool>,
    launcher_type: LauncherType,
    target: Option<String>,
    editable_name: Option<bool>,
    editable_icon: Option<bool>,
}

impl PrepareInstallOptions {
    /// Sets whether the dialog should be a modal.
    pub fn modal(mut self, modal: impl Into<Option<bool>>) -> Self {
        self.modal = modal.into();
        self
    }

    /// Sets the launcher type.
    pub fn launcher_type(mut self, launcher_type: LauncherType) -> Self {
        self.launcher_type = launcher_type;
        self
    }

    /// The URL for a [`LauncherType::WebApplication`] otherwise it is not
    /// needed.
    pub fn target<'a>(mut self, target: impl Into<Option<&'a str>>) -> Self {
        self.target = target.into().map(ToOwned::to_owned);
        self
    }

    /// Sets whether the name should be editable.
    pub fn editable_name(mut self, editable_name: impl Into<Option<bool>>) -> Self {
        self.editable_name = editable_name.into();
        self
    }

    /// Sets whether the icon should be editable.
    pub fn editable_icon(mut self, editable_icon: impl Into<Option<bool>>) -> Self {
        self.editable_icon = editable_icon.into();
        self
    }
}

/// The interface lets sandboxed applications install launchers like Web
/// Application from your browser or Steam.
///
/// Wrapper of the DBus interface: [`org.freedesktop.portal.DynamicLauncher`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-org.freedesktop.portal.DynamicLauncher).
#[derive(Debug)]
#[doc(alias = "org.freedesktop.portal.DynamicLauncher")]
pub struct DynamicLauncherProxy<'a>(Proxy<'a>);

impl<'a> DynamicLauncherProxy<'a> {
    /// Create a new instance of [`DynamicLauncherProxy`].
    pub async fn new() -> Result<DynamicLauncherProxy<'a>, Error> {
        let proxy = Proxy::new_desktop("org.freedesktop.portal.DynamicLauncher").await?;
        Ok(Self(proxy))
    }

    /// # Specifications
    ///
    /// See also [`PrepareInstall`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-DynamicLauncher.PrepareInstall).
    #[doc(alias = "PrepareInstall")]
    #[doc(alias = "xdp_portal_dynamic_launcher_prepare_install")]
    #[doc(alias = "xdp_portal_dynamic_launcher_prepare_install_finish")]
    pub async fn prepare_install(
        &self,
        parent_window: &WindowIdentifier,
        name: &str,
        icon: Icon,
        options: PrepareInstallOptions,
    ) -> Result<Request<(String, String)>, Error> {
        self.0
            .request(
                &options.handle_token,
                "PrepareInstall",
                &(parent_window, name, icon, &options),
            )
            .await
    }

    /// # Specifications
    ///
    /// See also [`RequestInstallToken`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-DynamicLauncher.RequestInstallToken).
    #[doc(alias = "RequestInstallToken")]
    #[doc(alias = "xdp_portal_dynamic_launcher_request_install_token")]
    pub async fn request_install_token(&self, name: &str, icon: Icon) -> Result<String, Error> {
        // No supported options for now
        let options: HashMap<&str, zvariant::Value<'_>> = HashMap::new();
        self.0
            .call::<String>("RequestInstallToken", &(name, icon, options))
            .await
    }

    /// # Specifications
    ///
    /// See also [`Install`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-DynamicLauncher.Install).
    #[doc(alias = "Install")]
    #[doc(alias = "xdp_portal_dynamic_launcher_install")]
    pub async fn install(
        &self,
        token: &str,
        desktop_file_id: &str,
        desktop_entry: &str,
    ) -> Result<(), Error> {
        // No supported options for now
        let options: HashMap<&str, zvariant::Value<'_>> = HashMap::new();
        self.0
            .call::<()>("Install", &(token, desktop_file_id, desktop_entry, options))
            .await
    }

    /// # Specifications
    ///
    /// See also [`Uninstall`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-DynamicLauncher.Uninstall).
    #[doc(alias = "Uninstall")]
    #[doc(alias = "xdp_portal_dynamic_launcher_uninstall")]
    pub async fn uninstall(&self, desktop_file_id: &str) -> Result<(), Error> {
        // No supported options for now
        let options: HashMap<&str, zvariant::Value<'_>> = HashMap::new();
        self.0
            .call::<()>("Uninstall", &(desktop_file_id, options))
            .await
    }

    /// # Specifications
    ///
    /// See also [`GetDesktopEntry`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-DynamicLauncher.GetDesktopEntry).
    #[doc(alias = "GetDesktopEntry")]
    #[doc(alias = "xdp_portal_dynamic_launcher_get_desktop_entry")]
    pub async fn desktop_entry(&self, desktop_file_id: &str) -> Result<String, Error> {
        self.0.call("GetDesktopEntry", &(desktop_file_id)).await
    }

    /// # Specifications
    ///
    /// See also [`GetIcon`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-DynamicLauncher.GetIcon).
    #[doc(alias = "GetIcon")]
    #[doc(alias = "xdp_portal_dynamic_launcher_get_icon")]
    pub async fn icon(&self, desktop_file_id: &str) -> Result<LauncherIcon, Error> {
        self.0.call("GetIcon", &(desktop_file_id)).await
    }

    /// # Specifications
    ///
    /// See also [`Launch`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-method-org-freedesktop-portal-DynamicLauncher.Launch).
    #[doc(alias = "Launch")]
    #[doc(alias = "xdp_portal_dynamic_launcher_launch")]
    pub async fn launch(&self, desktop_file_id: &str) -> Result<(), Error> {
        // TODO: handle activation_token
        let options: HashMap<&str, zvariant::Value<'_>> = HashMap::new();
        self.0.call("Launch", &(desktop_file_id, &options)).await
    }

    /// # Specifications
    ///
    /// See also [`SupportedLauncherTypes`](https://flatpak.github.io/xdg-desktop-portal/index.html#gdbus-property-org-freedesktop-portal-DynamicLauncher.SupportedLauncherTypes).
    #[doc(alias = "SupportedLauncherTypes")]
    pub async fn supported_launcher_types(&self) -> Result<BitFlags<LauncherType>, Error> {
        self.0
            .property::<BitFlags<LauncherType>>("SupportedLauncherTypes")
            .await
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_icon_signature() {
        let signature = LauncherIcon::signature();
        assert_eq!(signature.as_str(), "(vsu)");

        let icon = vec![IconType::Png];
        assert_eq!(serde_json::to_string(&icon).unwrap(), "[\"png\"]");
    }
}
