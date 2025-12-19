//! Set the native app icon at runtime.
//!
//! TODO(emilk): port this to [`winit`].

use std::sync::Arc;

use egui::IconData;

pub struct AppTitleIconSetter {
    title: String,
    icon_data: Option<Arc<IconData>>,
    status: AppIconStatus,
}

impl AppTitleIconSetter {
    pub fn new(title: String, mut icon_data: Option<Arc<IconData>>) -> Self {
        if let Some(icon) = &icon_data
            && **icon == IconData::default()
        {
            icon_data = None;
        }

        Self {
            title,
            icon_data,
            status: AppIconStatus::NotSetTryAgain,
        }
    }

    /// Call once per frame; we will set the icon when we can.
    pub fn update(&mut self) {
        if self.status == AppIconStatus::NotSetTryAgain {
            self.status = set_title_and_icon(&self.title, self.icon_data.as_deref());
        }
    }
}

/// In which state the app icon is (as far as we know).
#[derive(PartialEq, Eq)]
enum AppIconStatus {
    /// We did not set it or failed to do it. In any case we won't try again.
    NotSetIgnored,

    /// We haven't set the icon yet, we should try again next frame.
    ///
    /// This can happen repeatedly due to lazy window creation on some platforms.
    NotSetTryAgain,

    /// We successfully set the icon and it should be visible now.
    #[allow(clippy::allow_attributes, dead_code)] // Not used on Linux
    Set,
}

/// Sets app icon at runtime.
///
/// By setting the icon at runtime and not via resource files etc. we ensure that we'll get the chance
/// to set the same icon when the process/window is started from python (which sets its own icon ahead of us!).
///
/// Since window creation can be lazy, call this every frame until it's either successfully or gave up.
/// (See [`AppIconStatus`])
fn set_title_and_icon(_title: &str, _icon_data: Option<&IconData>) -> AppIconStatus {
    profiling::function_scope!();

    #[cfg(target_os = "windows")]
    {
        if let Some(icon_data) = _icon_data {
            return set_app_icon_windows(icon_data);
        }
    }

    #[cfg(target_os = "macos")]
    return set_title_and_icon_mac(_title, _icon_data);

    #[allow(clippy::allow_attributes, unreachable_code)]
    AppIconStatus::NotSetIgnored
}

/// Set icon for Windows applications.
#[cfg(target_os = "windows")]
#[expect(unsafe_code)]
fn set_app_icon_windows(icon_data: &IconData) -> AppIconStatus {
    use crate::icon_data::IconDataExt as _;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateIconFromResourceEx, GetSystemMetrics, HICON, ICON_BIG, ICON_SMALL, LR_DEFAULTCOLOR,
        SM_CXICON, SM_CXSMICON, SendMessageW, WM_SETICON,
    };

    // We would get fairly far already with winit's `set_window_icon` (which is exposed to eframe) actually!
    // However, it only sets ICON_SMALL, i.e. doesn't allow us to set a higher resolution icon for the task bar.
    // Also, there is scaling issues, detailed below.

    // TODO(andreas): This does not set the task bar icon for when our application is started from python.
    //      Things tried so far:
    //      * Querying for an owning window and setting icon there (there doesn't seem to be an owning window)
    //      * using undocumented SetConsoleIcon method (successfully queried via GetProcAddress)

    // SAFETY: WinApi function without side-effects.
    let window_handle = unsafe { GetActiveWindow() };
    if window_handle.is_null() {
        // The Window isn't available yet. Try again later!
        return AppIconStatus::NotSetTryAgain;
    }

    fn create_hicon_with_scale(unscaled_image: &image::RgbaImage, target_size: i32) -> HICON {
        let image_scaled = image::imageops::resize(
            unscaled_image,
            target_size as _,
            target_size as _,
            image::imageops::Lanczos3,
        );

        // Creating transparent icons with WinApi is a huge mess.
        // We'd need to go through CreateIconIndirect's ICONINFO struct which then
        // takes a mask HBITMAP and a color HBITMAP and creating each of these is pain.
        // Instead we workaround this by creating a png which CreateIconFromResourceEx magically understands.
        // This is a pretty horrible hack as we spend a lot of time encoding, but at least the code is a lot shorter.
        let mut image_scaled_bytes: Vec<u8> = Vec::new();
        if image_scaled
            .write_to(
                &mut std::io::Cursor::new(&mut image_scaled_bytes),
                image::ImageFormat::Png,
            )
            .is_err()
        {
            return std::ptr::null_mut();
        }

        // SAFETY: Creating an HICON which should be readonly on our data.
        unsafe {
            CreateIconFromResourceEx(
                image_scaled_bytes.as_mut_ptr(),
                image_scaled_bytes.len() as u32,
                1,           // Means this is an icon, not a cursor.
                0x00030000,  // Version number of the HICON
                target_size, // Note that this method can scale, but it does so *very* poorly. So let's avoid that!
                target_size,
                LR_DEFAULTCOLOR,
            )
        }
    }

    let unscaled_image = match icon_data.to_image() {
        Ok(unscaled_image) => unscaled_image,
        Err(err) => {
            log::warn!("Invalid icon: {err}");
            return AppIconStatus::NotSetIgnored;
        }
    };

    // Only setting ICON_BIG with the icon size for big icons (SM_CXICON) works fine
    // but the scaling it does then for the small icon is pretty bad.
    // Instead we set the correct sizes manually and take over the scaling ourselves.
    // For this to work we first need to set the big icon and then the small one.
    //
    // Note that ICON_SMALL may be used even if we don't render a title bar as it may be used in alt+tab!
    {
        // SAFETY: WinAPI getter function with no known side effects.
        let icon_size_big = unsafe { GetSystemMetrics(SM_CXICON) };
        let icon_big = create_hicon_with_scale(&unscaled_image, icon_size_big);
        if icon_big.is_null() {
            log::warn!("Failed to create HICON (for big icon) from embedded png data.");
            return AppIconStatus::NotSetIgnored; // We could try independently with the small icon but what's the point, it would look bad!
        } else {
            // SAFETY: Unsafe WinApi function, takes objects previously created with WinAPI, all checked for null prior.
            unsafe {
                SendMessageW(
                    window_handle,
                    WM_SETICON,
                    ICON_BIG as usize,
                    icon_big as isize,
                );
            }
        }
    }
    {
        // SAFETY: WinAPI getter function with no known side effects.
        let icon_size_small = unsafe { GetSystemMetrics(SM_CXSMICON) };
        let icon_small = create_hicon_with_scale(&unscaled_image, icon_size_small);
        if icon_small.is_null() {
            log::warn!("Failed to create HICON (for small icon) from embedded png data.");
            return AppIconStatus::NotSetIgnored;
        } else {
            // SAFETY: Unsafe WinApi function, takes objects previously created with WinAPI, all checked for null prior.
            unsafe {
                SendMessageW(
                    window_handle,
                    WM_SETICON,
                    ICON_SMALL as usize,
                    icon_small as isize,
                );
            }
        }
    }

    // It _probably_ worked out.
    AppIconStatus::Set
}

/// Set icon & app title for `MacOS` applications.
#[cfg(target_os = "macos")]
#[expect(unsafe_code)]
fn set_title_and_icon_mac(title: &str, icon_data: Option<&IconData>) -> AppIconStatus {
    use crate::icon_data::IconDataExt as _;
    profiling::function_scope!();

    use objc2::ClassType as _;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSString;

    // Do NOT use png even though creating `NSImage` from it is much easier than from raw images data!
    //
    // Some MacOS versions have a bug where creating an `NSImage` from a png will cause it to load an arbitrary `libpng.dylib`.
    // If this dylib isn't the right version, the application will crash with SIGBUS.
    // For details see https://github.com/emilk/egui/issues/7155
    let image = if let Some(icon_data) = icon_data {
        match icon_data.to_image() {
            Ok(image) => Some(image),
            Err(err) => {
                log::warn!("Failed to read icon data: {err}");
                return AppIconStatus::NotSetIgnored;
            }
        }
    } else {
        None
    };

    // TODO(madsmtm): Move this into `objc2-app-kit`
    unsafe extern "C" {
        static NSApp: Option<&'static NSApplication>;
    }

    // SAFETY: we don't do anything dangerous here
    unsafe {
        let Some(app) = NSApp else {
            log::debug!("NSApp is null");
            return AppIconStatus::NotSetIgnored;
        };

        if let Some(image) = image {
            use objc2_app_kit::{NSBitmapImageRep, NSDeviceRGBColorSpace};
            use objc2_foundation::NSSize;

            log::trace!(
                "NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel"
            );
            let Some(image_rep) = NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bytesPerRow_bitsPerPixel(
                NSBitmapImageRep::alloc(),
                [image.as_raw().as_ptr().cast_mut()].as_mut_ptr(),
                image.width() as isize,
                image.height() as isize,
                8, // bits per sample
                4, // samples per pixel
                true, // has alpha
                false, // is not planar
                NSDeviceRGBColorSpace,
                (image.width() * 4) as isize, // bytes per row
                32 // bits per pixel
            ) else {
                log::warn!("Failed to create NSBitmapImageRep from app icon data.");
                return AppIconStatus::NotSetIgnored;
            };

            log::trace!("NSImage::initWithSize");
            let app_icon = NSImage::initWithSize(
                NSImage::alloc(),
                NSSize::new(image.width() as f64, image.height() as f64),
            );
            log::trace!("NSImage::addRepresentation");
            app_icon.addRepresentation(&image_rep);

            profiling::scope!("setApplicationIconImage_");
            log::trace!("setApplicationIconImage…");
            app.setApplicationIconImage(Some(&app_icon));
        }

        // Change the title in the top bar - for python processes this would be again "python" otherwise.
        if let Some(main_menu) = app.mainMenu()
            && let Some(item) = main_menu.itemAtIndex(0)
            && let Some(app_menu) = item.submenu()
        {
            profiling::scope!("setTitle_");
            app_menu.setTitle(&NSString::from_str(title));
        }

        // The title in the Dock apparently can't be changed.
        // At least these people didn't figure it out either:
        // https://stackoverflow.com/questions/69831167/qt-change-application-title-dynamically-on-macos
        // https://stackoverflow.com/questions/28808226/changing-cocoa-app-icon-title-and-menu-labels-at-runtime
    }

    AppIconStatus::Set
}
