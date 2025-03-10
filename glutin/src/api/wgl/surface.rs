//! A wrapper around `HWND` used for GL operations.

use std::fmt;
use std::io::Error as IoError;
use std::marker::PhantomData;
use std::num::NonZeroU32;

use raw_window_handle::RawWindowHandle;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::Graphics::{Gdi as gdi, OpenGL as gl};

use crate::config::GetGlConfig;
use crate::display::{DisplayFeatures, GetGlDisplay};
use crate::error::{ErrorKind, Result};
use crate::prelude::*;
use crate::private::Sealed;
use crate::surface::{
    AsRawSurface, GlSurface, PbufferSurface, PixmapSurface, RawSurface, SurfaceAttributes,
    SurfaceTypeTrait, SwapInterval, WindowSurface,
};

use super::config::Config;
use super::context::PossiblyCurrentContext;
use super::display::Display;

impl Display {
    pub(crate) unsafe fn create_pixmap_surface(
        &self,
        _config: &Config,
        _surface_attributes: &SurfaceAttributes<PixmapSurface>,
    ) -> Result<Surface<PixmapSurface>> {
        Err(ErrorKind::NotSupported("pixmaps are not implemented with WGL").into())
    }

    pub(crate) unsafe fn create_pbuffer_surface(
        &self,
        _config: &Config,
        _surface_attributes: &SurfaceAttributes<PbufferSurface>,
    ) -> Result<Surface<PbufferSurface>> {
        Err(ErrorKind::NotSupported("pbuffers are not implemented with WGL").into())
    }

    pub(crate) unsafe fn create_window_surface(
        &self,
        config: &Config,
        surface_attributes: &SurfaceAttributes<WindowSurface>,
    ) -> Result<Surface<WindowSurface>> {
        let hwnd = match surface_attributes.raw_window_handle.as_ref().unwrap() {
            handle @ RawWindowHandle::Win32(window_handle) => {
                let _ = unsafe { config.apply_on_native_window(handle) };
                window_handle.hwnd as HWND
            },
            _ => {
                return Err(
                    ErrorKind::NotSupported("provided native window is not supported").into()
                )
            },
        };

        let surface =
            Surface { display: self.clone(), config: config.clone(), hwnd, _ty: PhantomData };

        Ok(surface)
    }
}

/// A Wrapper around `HWND`.
pub struct Surface<T: SurfaceTypeTrait> {
    display: Display,
    config: Config,
    pub(crate) hwnd: HWND,
    _ty: PhantomData<T>,
}

impl<T: SurfaceTypeTrait> Drop for Surface<T> {
    fn drop(&mut self) {
        // This line intentionally left blank.
    }
}

impl<T: SurfaceTypeTrait> GlSurface<T> for Surface<T> {
    type Context = PossiblyCurrentContext;
    type SurfaceType = T;

    fn buffer_age(&self) -> u32 {
        0
    }

    fn width(&self) -> Option<u32> {
        None
    }

    fn height(&self) -> Option<u32> {
        None
    }

    fn is_single_buffered(&self) -> bool {
        self.config.is_single_buffered()
    }

    fn swap_buffers(&self, _context: &Self::Context) -> Result<()> {
        unsafe {
            let hdc = gdi::GetDC(self.hwnd);
            if gl::SwapBuffers(hdc) == 0 {
                Err(IoError::last_os_error().into())
            } else {
                Ok(())
            }
        }
    }

    fn set_swap_interval(&self, _context: &Self::Context, interval: SwapInterval) -> Result<()> {
        let interval = match interval {
            SwapInterval::DontWait => 0,
            SwapInterval::Wait(n) => n.get(),
        };

        let res = match self.display.inner.wgl_extra {
            Some(extra) if self.display.inner.features.contains(DisplayFeatures::SWAP_CONTROL) => unsafe {
                extra.SwapIntervalEXT(interval as _)
            },
            _ => {
                return Err(
                    ErrorKind::NotSupported("swap contol extrensions are not supported").into()
                )
            },
        };

        if res == 0 {
            Err(IoError::last_os_error().into())
        } else {
            Ok(())
        }
    }

    fn is_current(&self, context: &Self::Context) -> bool {
        context.is_current()
    }

    fn is_current_draw(&self, context: &Self::Context) -> bool {
        context.is_current()
    }

    fn is_current_read(&self, context: &Self::Context) -> bool {
        context.is_current()
    }

    fn resize(&self, _context: &Self::Context, _width: NonZeroU32, _height: NonZeroU32) {
        // This isn't supported with WGL.
    }
}

impl<T: SurfaceTypeTrait> fmt::Debug for Surface<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("config", &self.config.inner.pixel_format_index)
            .field("hwnd", &self.hwnd)
            .finish()
    }
}

impl<T: SurfaceTypeTrait> AsRawSurface for Surface<T> {
    fn raw_surface(&self) -> RawSurface {
        RawSurface::Wgl(self.hwnd as _)
    }
}

impl<T: SurfaceTypeTrait> GetGlConfig for Surface<T> {
    type Target = Config;

    fn config(&self) -> Self::Target {
        self.config.clone()
    }
}

impl<T: SurfaceTypeTrait> GetGlDisplay for Surface<T> {
    type Target = Display;

    fn display(&self) -> Self::Target {
        self.display.clone()
    }
}

impl<T: SurfaceTypeTrait> Sealed for Surface<T> {}
