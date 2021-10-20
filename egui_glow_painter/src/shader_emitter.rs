pub(crate) enum GlslIo {
    In,
    Out
}
pub(crate) enum ShaderStage{
    Fragment,
    Vertex,
}
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
enum ShaderVersion {
    Gl120,
    Gl140,
    Es100,
    Es300,
}
pub(crate) struct Emitter{
    in_out_old_style:bool,
    shader_version:ShaderVersion,
    stage:ShaderStage,
    text_buffer:String
}
pub(crate) enum Ty{
    Vec2,
    Vec3,
    Vec4,
}

impl ToString for Ty {
    fn to_string(&self)->String{
        match self {
            Ty::Vec2 => {"vec2"}
            Ty::Vec3 => {"vec3"}
            Ty::Vec4 => {"vec4"}
        }.to_owned()
    }
}
pub(crate) struct TyAndIdent{
    pub ty:Ty,
    pub identifier:String
}
impl ToString for TyAndIdent{
    fn to_string(&self) -> String {
        format!("{} {}",self.ty.to_string(),self.identifier)
    }
}
impl Emitter{
    fn new(stage:ShaderStage, in_out_old_style:bool, shader_version:ShaderVersion) -> Emitter {
        Self{
            in_out_old_style,
            shader_version,
            stage,
            text_buffer: String::new()
        }
    }
    ///stage is vertex or fragment
    ///
    ///when in_out_old_style true
    /// in -> attribute ,varying
    ///
    /// out -> varying,
    ///
    fn add_in_out(&mut self,direction:GlslIo,ty_and_identifier:TyAndIdent){
        let ty_and_identifier=ty_and_identifier.to_string();
        match self.stage {
        ShaderStage::Fragment => {
            if self.in_out_old_style{
               if let GlslIo::In=direction{
                    self.text_buffer+=&format!("varying {};",ty_and_identifier);
                }
            }else{
                match direction {
                    GlslIo::In => {self.text_buffer+= &format!("in {};", ty_and_identifier) }
                    GlslIo::Out => {self.text_buffer.push_str(&format!("out {};",ty_and_identifier))}
                }
            }
        }
        ShaderStage::Vertex => {
            if self.in_out_old_style{
                match direction {
                    GlslIo::In => {
                        self.text_buffer+=&format!("attribute {};",ty_and_identifier);
                    }
                    GlslIo::Out => {
                        self.text_buffer+=&format!("varying {};",ty_and_identifier);
                    }
                }
            }else{
                match direction {
                    GlslIo::In => {self.text_buffer+= &format!("in {};", ty_and_identifier) }
                    GlslIo::Out => {self.text_buffer.push_str(&format!("out {};",ty_and_identifier))}
                }
            }
        }
    }
    }
}