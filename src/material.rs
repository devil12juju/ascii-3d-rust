use std::{collections::HashMap, fs, path::{Path, PathBuf}};
use super::V;

pub fn linear(v:f32)->f32 {
    let v=v.clamp(0.0,1.0);
    if v<=0.04045 {v/12.92} else {((v+0.055)/1.055).powf(2.4)}
}
pub struct Texture {
    pixels:image::RgbImage,
    offset:V,
    scale:V,
    clamp:bool,
}
impl Texture {
    fn load(path:&Path,offset:V,scale:V,clamp:bool)->Result<Self,String> {
        let mut reader=image::ImageReader::open(path).map_err(|e|format!("{} : {e}",path.display()))?
            .with_guessed_format().map_err(|e|e.to_string())?;
        let mut limits=image::Limits::default();
        limits.max_image_width=Some(16384); limits.max_image_height=Some(16384);
        limits.max_alloc=Some(256*1024*1024); reader.limits(limits);
        let pixels=reader.decode().map_err(|e|format!("{} : {e}",path.display()))?.to_rgb8();
        Ok(Self{pixels,offset,scale,clamp})
    }
    pub fn sample(&self,uv:V)->V {
        let coordinate=|x:f32|if self.clamp {x.clamp(0.0,1.0)} else {x.rem_euclid(1.0)};
        let u=coordinate(uv.x*self.scale.x+self.offset.x);
        // OBJ V starts at the bottom, image rows at the top.
        let v=coordinate(uv.y*self.scale.y+self.offset.y);
        let x=u*(self.pixels.width()-1) as f32;
        let y=(1.0-v)*(self.pixels.height()-1) as f32;
        let x0=x.floor() as u32; let y0=y.floor() as u32;
        let x1=(x0+1).min(self.pixels.width()-1); let y1=(y0+1).min(self.pixels.height()-1);
        let color=|x,y| {let p=self.pixels.get_pixel(x,y).0; V::new(linear(p[0] as f32/255.0),linear(p[1] as f32/255.0),linear(p[2] as f32/255.0))};
        let a=color(x0,y0)*(1.0-(x-x0 as f32))+color(x1,y0)*(x-x0 as f32);
        let b=color(x0,y1)*(1.0-(x-x0 as f32))+color(x1,y1)*(x-x0 as f32);
        a*(1.0-(y-y0 as f32))+b*(y-y0 as f32)
    }
}
pub struct Material {
    pub diffuse:V,
    pub specular:V,
    pub shininess:Option<f32>,
    pub texture:Option<Texture>,
}
impl Default for Material {
    fn default()->Self {Self{diffuse:V::new(1.0,1.0,1.0),specular:V::new(1.0,1.0,1.0),shininess:None,texture:None}}
}
pub fn resolve(parent:&Path,name:&str)->PathBuf {
    parent.join(name.trim().trim_matches('"'))
}
fn values(text:&str)->Result<Vec<f32>,String> {
    let v=text.split_whitespace().map(|x|x.parse::<f32>()).collect::<Result<Vec<_>,_>>().map_err(|_|"Invalid MTL number")?;
    if v.iter().any(|v|!v.is_finite()) {return Err("Non-finite MTL number".into());} Ok(v)
}
fn color(text:&str)->Result<V,String> {
    let v=values(text)?;
    if v.len()!=3 || v.iter().any(|v|*v<0.0 || *v>1.0) {return Err("MTL color requires three numbers between 0 and 1".into());}
    Ok(V::new(v[0],v[1],v[2]))
}
fn texture(text:&str,parent:&Path)->Result<Texture,String> {
    let words:Vec<_>=text.split_whitespace().collect(); let mut i=0;
    let mut offset=V::default(); let mut scale=V::new(1.0,1.0,1.0); let mut clamp=false;
    while i<words.len() && words[i].starts_with('-') {
        let option=words[i]; i+=1;
        match option {
            "-o"|"-s"|"-t"=>{
                let mut v=if option=="-s" {[1.0;3]} else {[0.0;3]}; let mut count=0;
                while count<3 && i<words.len() {
                    if let Ok(n)=words[i].parse::<f32>() {
                        if !n.is_finite() {return Err("Non-finite texture transform".into());}
                        v[count]=n;count+=1;i+=1;
                    } else {break;}
                }
                if count==0 {return Err(format!("Missing value for {option}"));}
                if option=="-o" {offset=V::new(v[0],v[1],v[2]);}
                if option=="-s" {scale=V::new(v[0],v[1],v[2]);}
            }
            "-clamp"=>{clamp=*words.get(i).ok_or("Missing -clamp value")?=="on";i+=1;},
            "-mm"=>{if i+2>words.len() {return Err("Missing -mm values".into());}i+=2;},
            "-blendu"|"-blendv"|"-bm"|"-boost"|"-texres"|"-imfchan"|"-type"=>{if i>=words.len() {return Err(format!("Missing {option} value"));}i+=1;},
            _=>return Err(format!("Unsupported map_Kd option: {option}")),
        }
    }
    if i==words.len() {return Err("Missing texture path".into());}
    Texture::load(&resolve(parent,&words[i..].join(" ")),offset,scale,clamp)
}
pub fn load(path:&Path)->Result<HashMap<String,Material>,String> {
    let source=fs::read_to_string(path).map_err(|e|format!("Cannot read material library {}: {e}",path.display()))?;
    let mut materials=HashMap::new(); let mut name=String::new();
    for (index,line) in source.lines().enumerate() {
        let line=line.split('#').next().unwrap_or("").trim();
        let (key,text)=line.split_once(char::is_whitespace).unwrap_or((line,"")); let text=text.trim();
        if key=="newmtl" {
            if text.is_empty() {return Err(format!("Missing material name at line {}",index+1));}
            name=text.to_string();materials.insert(name.clone(),Material::default());continue;
        }
        let Some(m)=materials.get_mut(&name) else {continue;};
        let result:Result<(),String>=(|| {
            match key {
                "Kd"=>m.diffuse=color(text)?,
                "Ks"=>m.specular=color(text)?,
                "Ns"=>{let v=values(text)?;if v.len()!=1 || v[0]<0.0 {return Err("Invalid Ns value".into());}m.shininess=Some(v[0].clamp(1.0,1000.0));},
                "map_Kd"=>match texture(text,path.parent().unwrap_or(Path::new("."))) {
                    Ok(t)=>m.texture=Some(t),
                    Err(e)=>eprintln!("Warning: texture for material {name} could not be loaded: {e}"),
                },
                _=>{}
            } Ok(())
        })();
        result.map_err(|e|format!("{} at line {}: {e}",path.display(),index+1))?;
    }
    Ok(materials)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn png_jpeg_paths_with_spaces_and_material_properties() {
        let parent=Path::new("work/material-test"); fs::create_dir_all(parent).unwrap();
        let pixels=image::RgbImage::from_pixel(4,4,image::Rgb([255,0,0]));
        pixels.save(parent.join("red texture.png")).unwrap();
        pixels.save(parent.join("red texture.jpg")).unwrap();
        fs::write(parent.join("example.mtl"),"newmtl painted\nKd 0.8 0.5 0.2\nKs 0.2 0.3 0.4\nNs 80\nmap_Kd -clamp on -o 0 0 -s 1 1 \"red texture.png\"\nnewmtl jpeg\nmap_Kd red texture.jpg\n").unwrap();
        let materials=load(&parent.join("example.mtl")).unwrap();
        let mat=materials.get("painted").unwrap();
        assert_eq!(mat.shininess,Some(80.0));
        assert!((mat.diffuse.x-0.8).abs()<0.001 && (mat.specular.z-0.4).abs()<0.001);
        for name in ["painted","jpeg"] {
            let c=materials.get(name).unwrap().texture.as_ref().unwrap().sample(V::new(0.5,0.5,0.0));
            assert!(c.x>0.9 && c.y<0.01 && c.z<0.01);
        }
    }
}
