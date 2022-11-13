#![allow(unsafe_code)]

use std::convert::TryInto;

/// Helper for parsing and interpreting the OpenGL shader version.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ShaderVersion {
    Gl120,

    /// OpenGL 1.4 or later
    Gl140,

    /// e.g. WebGL1
    Es100,

    /// e.g. WebGL2
    Es300,
}

impl ShaderVersion {
    pub fn get(gl: &glow::Context) -> Self {
        use glow::HasContext as _;
        let shading_lang_string =
            unsafe { gl.get_parameter_string(glow::SHADING_LANGUAGE_VERSION) };
        let shader_version = Self::parse(&shading_lang_string);
        tracing::debug!(
            "Shader version: {:?} ({:?}).",
            shader_version,
            shading_lang_string
        );
        shader_version
    }

    #[inline]
    pub(crate) fn parse(glsl_ver: &str) -> Self {
        let start = glsl_ver.find(|c| char::is_ascii_digit(&c)).unwrap();
        let es = glsl_ver[..start].contains(" ES ");
        let ver = glsl_ver[start..]
            .split_once(' ')
            .map_or(&glsl_ver[start..], |x| x.0);
        let [maj, min]: [u8; 2] = ver
            .splitn(3, '.')
            .take(2)
            .map(|x| x.parse().unwrap_or_default())
            .collect::<Vec<u8>>()
            .try_into()
            .unwrap();
        if es {
            if maj >= 3 {
                Self::Es300
            } else {
                Self::Es100
            }
        } else if maj > 1 || (maj == 1 && min >= 40) {
            Self::Gl140
        } else {
            Self::Gl120
        }
    }

    /// Goes on top of the shader.
    pub fn version_declaration(&self) -> &'static str {
        match self {
            Self::Gl120 => "#version 120\n",
            Self::Gl140 => "#version 140\n",
            Self::Es100 => "#version 100\n",
            Self::Es300 => "#version 300 es\n",
        }
    }

    /// If true, use `in/out`. If `false`, use `varying` and `gl_FragColor`.
    pub fn is_new_shader_interface(&self) -> bool {
        match self {
            Self::Gl120 | Self::Es100 => false,
            Self::Es300 | Self::Gl140 => true,
        }
    }

    pub fn is_embedded(&self) -> bool {
        match self {
            Self::Gl120 | Self::Gl140 => false,
            Self::Es100 | Self::Es300 => true,
        }
    }
}

#[test]
fn test_shader_version() {
    use ShaderVersion::{Es100, Es300, Gl120, Gl140};
    for (s, v) in [
        ("1.2 OpenGL foo bar", Gl120),
        ("3.0", Gl140),
        ("0.0", Gl120),
        ("OpenGL ES GLSL 3.00 (WebGL2)", Es300),
        ("OpenGL ES GLSL 1.00 (WebGL)", Es100),
        ("OpenGL ES GLSL ES 1.00 foo bar", Es100),
        ("WebGL GLSL ES 3.00 foo bar", Es300),
        ("WebGL GLSL ES 3.00", Es300),
        ("WebGL GLSL ES 1.0 foo bar", Es100),
    ] {
        assert_eq!(ShaderVersion::parse(s), v);
    }
}
