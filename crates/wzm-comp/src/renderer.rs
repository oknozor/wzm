use smithay::backend::allocator::dmabuf::Dmabuf;
use smithay::backend::renderer::gles::{GlesFrame, GlesRenderer, GlesTexture};
use smithay::backend::renderer::{
    Bind, ExportMem, ImportAll, ImportMem, Offscreen, Renderer, Texture,
};

// Shamelessly stolen from NIRI

/// Trait with our main renderer requirements to save on the typing.
pub trait WzmRenderer:
    ImportAll
    + ImportMem
    + ExportMem
    + Bind<Dmabuf>
    + Offscreen<GlesTexture>
    + Renderer<TextureId = Self::WzmTextureId, Error = Self::WzmError>
    + AsGlesRenderer
{
    // Associated types to work around the instability of associated type bounds.
    type WzmTextureId: Texture + Clone + 'static;
    type WzmError: std::error::Error
        + Send
        + Sync
        + From<<GlesRenderer as Renderer>::Error>
        + 'static;
}

impl<R> WzmRenderer for R
where
    R: ImportAll + ImportMem + ExportMem + Bind<Dmabuf> + Offscreen<GlesTexture> + AsGlesRenderer,
    R::TextureId: Texture + Clone + 'static,
    R::Error: std::error::Error + Send + Sync + From<<GlesRenderer as Renderer>::Error> + 'static,
{
    type WzmTextureId = R::TextureId;
    type WzmError = R::Error;
}

/// Trait for getting the underlying `GlesRenderer`.
pub trait AsGlesRenderer {
    fn as_gles_renderer(&mut self) -> &mut GlesRenderer;
}

impl AsGlesRenderer for GlesRenderer {
    fn as_gles_renderer(&mut self) -> &mut GlesRenderer {
        self
    }
}

/// Trait for getting the underlying `GlesFrame`.
pub trait AsGlesFrame<'frame>
where
    Self: 'frame,
{
    fn as_gles_frame(&mut self) -> &mut GlesFrame<'frame>;
}

impl<'frame> AsGlesFrame<'frame> for GlesFrame<'frame> {
    fn as_gles_frame(&mut self) -> &mut GlesFrame<'frame> {
        self
    }
}
