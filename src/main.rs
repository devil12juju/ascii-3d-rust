use std::{env, fs, io::{self, Write}, ops::{Add, Sub, Mul}, thread, time::{Duration, Instant}};
use std::{collections::{HashMap,HashSet},path::Path};
mod material;

#[derive(Clone, Copy, Default)]
struct V { x: f32, y: f32, z: f32 }
impl V {
    fn new(x:f32,y:f32,z:f32)->Self {Self{x,y,z}}
    fn dot(self,b:Self)->f32 {self.x*b.x+self.y*b.y+self.z*b.z}
    fn cross(self,b:Self)->Self {Self::new(self.y*b.z-self.z*b.y,self.z*b.x-self.x*b.z,self.x*b.y-self.y*b.x)}
    fn unit(self)->Self {self*(1.0/self.dot(self).sqrt().max(1e-8))}
    fn euler(self, angles:Self)->Self {
        let (s,c)=angles.x.to_radians().sin_cos();
        let p=Self::new(self.x,self.y*c-self.z*s,self.y*s+self.z*c);
        let (s,c)=angles.y.to_radians().sin_cos();
        let p=Self::new(p.x*c+p.z*s,p.y,-p.x*s+p.z*c);
        let (s,c)=angles.z.to_radians().sin_cos();
        Self::new(p.x*c-p.y*s,p.x*s+p.y*c,p.z)
    }
}
impl Add for V {type Output=Self; fn add(self,b:Self)->Self {Self::new(self.x+b.x,self.y+b.y,self.z+b.z)}}
impl Sub for V {type Output=Self; fn sub(self,b:Self)->Self {Self::new(self.x-b.x,self.y-b.y,self.z-b.z)}}
impl Mul<f32> for V {type Output=Self; fn mul(self,b:f32)->Self {Self::new(self.x*b,self.y*b,self.z*b)}}
impl V {fn product(self,b:Self)->Self {Self::new(self.x*b.x,self.y*b.y,self.z*b.z)}}

struct Face {indices:[usize;3],uv:[Option<usize>;3],material:Option<String>}
struct Mesh {vertices:Vec<V>,uv:Vec<V>,faces:Vec<Face>,materials:HashMap<String,material::Material>}
fn obj_index(text:&str,length:usize)->Result<usize,String> {
    let i=text.parse::<isize>().map_err(|_|"Invalid OBJ index")?;
    let i=if i>0 {i-1} else if i<0 {length as isize+i} else {return Err("OBJ indices cannot be zero".into());};
    if i<0 || i as usize>=length {return Err("OBJ index out of bounds".into());} Ok(i as usize)
}
impl Mesh {
    fn load(path:&str)->Result<Self,String> {
        Self::load_with_mtl(path,None)
    }
    fn load_with_mtl(path:&str,override_mtl:Option<&str>)->Result<Self,String> {
        let source=fs::read_to_string(path).map_err(|e|format!("Cannot read {path}: {e}"))?;
        let mut m=Self{vertices:vec![],uv:vec![],faces:vec![],materials:HashMap::new()};
        let parent=Path::new(path).parent().unwrap_or(Path::new("."));
        let mut material_name=None; let mut libraries=vec![];
        for (line_no,line) in source.lines().enumerate() {
            let mut parts=line.split('#').next().unwrap_or("").split_whitespace();
            let error=||format!("Invalid OBJ at line {}",line_no+1);
            match parts.next() {
                Some("mtllib")=>{
                    let names:Vec<_>=parts.collect(); let joined=names.join(" ");
                    if material::resolve(parent,&joined).is_file() {libraries.push(material::resolve(parent,&joined));}
                    else {libraries.extend(names.iter().map(|n|material::resolve(parent,n)));}
                }
                Some("usemtl")=>{let name=parts.collect::<Vec<_>>().join(" ");material_name=Some(name);},
                Some("vt")=>{
                    let u=parts.next().ok_or_else(error)?.parse::<f32>().map_err(|_|error())?;
                    let v=parts.next().unwrap_or("0").parse::<f32>().map_err(|_|error())?;
                    if !u.is_finite() || !v.is_finite() {return Err(error());}
                    m.uv.push(V::new(u,v,0.0));
                }
                Some("v")=>{
                    let mut p=[0.0;3];
                    for v in &mut p {*v=parts.next().ok_or_else(error)?.parse::<f32>().map_err(|_|error())?;}
                    if p.iter().any(|v|!v.is_finite()) {return Err(error());}
                    m.vertices.push(V::new(p[0],p[1],p[2]));
                }
                Some("f")=>{
                    let mut face=vec![];
                    for token in parts {
                        let mut indices=token.split('/');
                        let vertex=obj_index(indices.next().unwrap(),m.vertices.len()).map_err(|_|error())?;
                        let uv=indices.next().filter(|s|!s.is_empty()).map(|s|obj_index(s,m.uv.len())).transpose().map_err(|_|error())?;
                        face.push((vertex,uv));
                    }
                    if face.len()<3 {return Err(error());}
                    // Triangle fan: triangles, quads and convex polygons.
                    for i in 1..face.len()-1 {
                        let f=[face[0],face[i],face[i+1]];
                        m.faces.push(Face{indices:f.map(|p|p.0),uv:f.map(|p|p.1),material:material_name.clone()});
                    }
                }
                _=>{}
            }
        }
        if m.faces.is_empty() {return Err("The model does not contain any faces.".into());}
        if let Some(explicit)=override_mtl {libraries=vec![Path::new(explicit).to_path_buf()];}
        else if libraries.is_empty() {
            for extension in ["mtl","mlt"] {
                let candidate=Path::new(path).with_extension(extension);
                if candidate.is_file() {libraries.push(candidate);break;}
            }
        }
        for library in libraries {
            let library=if !library.is_file() && library.extension().is_some_and(|e|e=="mtl") && library.with_extension("mlt").is_file() {library.with_extension("mlt")} else {library};
            match material::load(&library) {
                Ok(materials)=>m.materials.extend(materials),
                Err(e)=>{if override_mtl.is_some() {return Err(e);}eprintln!("Warning: {e}");}
            }
        }
        if m.materials.len()==1 && m.faces.iter().all(|f|f.material.is_none()) {
            let name=m.materials.keys().next().unwrap().clone();
            for face in &mut m.faces {face.material=Some(name.clone());}
        }
        let mut warned=HashSet::new();
        for face in &m.faces {
            if let Some(name)=&face.material {
                if !m.materials.contains_key(name) && warned.insert(name.clone()) {eprintln!("Warning: material {name} was not found; using the base color.");}
                if m.materials.get(name).is_some_and(|m|m.texture.is_some()) && face.uv.iter().any(Option::is_none) && warned.insert(format!("uv:{name}")) {eprintln!("Warning: missing UV coordinates for {name}; texture skipped on these faces.");}
            }
        }
        let mut lo=m.vertices[0]; let mut hi=lo;
        for p in &m.vertices {
            lo=V::new(lo.x.min(p.x),lo.y.min(p.y),lo.z.min(p.z));
            hi=V::new(hi.x.max(p.x),hi.y.max(p.y),hi.z.max(p.z));
        }
        let center=(lo+hi)*0.5;
        let radius=m.vertices.iter().map(|p|(*p-center).dot(*p-center).sqrt()).fold(0.0_f32,f32::max);
        if radius<1e-8 || !radius.is_finite() {return Err("Invalid model size.".into());}
        for p in &mut m.vertices {*p=(*p-center)*(1.8/radius);}
        Ok(m)
    }
    fn torus()->Self {
        let (rows,cols)=(32,64); let mut m=Self{vertices:vec![],uv:vec![],faces:vec![],materials:HashMap::new()};
        for i in 0..rows {for j in 0..cols {
            let t=i as f32*std::f32::consts::TAU/rows as f32;
            let p=j as f32*std::f32::consts::TAU/cols as f32;
            m.vertices.push(V::new((1.15+0.45*t.cos())*p.cos(),(1.15+0.45*t.cos())*p.sin(),0.45*t.sin()));
        }}
        for i in 0..rows {for j in 0..cols {
            let a=i*cols+j; let b=((i+1)%rows)*cols+j;
            let c=((i+1)%rows)*cols+(j+1)%cols; let d=i*cols+(j+1)%cols;
            for indices in [[a,b,c],[a,c,d]] {m.faces.push(Face{indices,uv:[None;3],material:None});}
        }} m
    }
}
struct Light {position:V,color:V}
struct Options {
    model:Option<String>,lights:Vec<Light>,color:V,width:usize,height:usize,
    frames:Option<usize>,mono:bool,scale:f32,zoom:f32,position:V,offset:(f32,f32),
    margins:[usize;4],rotation:V,speed:f32,fps:usize,distance:f32,aspect:f32,
    ambient:f32,specular:f32,shininess:f32,chars:String,auto_size:bool,mtl:Option<String>,textures:bool,shininess_override:bool,
    pitch_limit:Option<f32>,roll_limit:Option<f32>,
}
const HELP:&str = "ASCII 3D Renderer in Rust

MODEL AND FRAMING
  --model file.obj           Load an OBJ (default: animated torus)
  --mtl file.mtl             Explicit material library override
  --no-textures              Disable image textures; keep material colors
  --width N --height N       Fixed dimensions (fallback: 100 x 40)
  --size WxH                Example: --size 120x45
  --auto-size               Follow the terminal size (default)
  --fixed-size              Keep the configured dimensions
  --scale N                 Model size (default: 1)
  --zoom N                  Projection magnification (default: 1)
  --position x,y,z           Model position in 3D (default: 0,0,0)
  --offset x,y               Screen offset in characters: right, down
  --center                  Reset position and offset; enable auto-size
  --margin N                Same margin on all sides (default: 0)
  --margins L,T,R,B          Left, top, right, bottom margins
  --distance N              Camera distance from origin (default: 5)
  --aspect N                Character width/height ratio (default: 0.5)

ANIMATION
  --rotation x,y,z           Initial rotation in degrees (default: 0,0,0)
  --pitch-limit N            Clamp pitch (X) to -N..+N degrees; 0 locks it
  --roll-limit N             Clamp roll (Z) to -N..+N degrees; 0 locks it
  --speed N                 Rotation speed in radians/second (default: 0.6)
  --static                  Disable animated rotation
  --fps N                   Target frame rate, 1..120 (default: 30)
  --once                    Render one frame
  --frames N                Render N frames, then exit

APPEARANCE AND LIGHTING
  --mono / --monochrome      ASCII output without colors
  --color r,g,b              Model tint, each channel 0..255
  --light x,y,z:r,g,b         Point light (repeatable)
  --ambient N               Ambient light, 0..2 (default: 0.055)
  --specular N              Reflection intensity, 0..2 (default: 0.35)
  --shininess N             Reflection concentration, 1..256 (default: 32)
  --chars TEXT              ASCII palette from dark to bright
  --help                    Show this help

Example: Run --model models\\cube.obj --center --scale 0.7 --margin 3 --mono
Press Q or Escape to exit on Windows.
Options apply in order: --size fixes dimensions; --center or --auto-size
enables live resizing again. One row and column are reserved to avoid scrolling.";
fn number(s:&str)->Result<f32,String> {
    let n=s.parse::<f32>().map_err(|_|"A number is required.".to_string())?;
    if !n.is_finite() {return Err("A finite number is required.".into());} Ok(n)
}
fn triple(s:&str)->Result<V,String> {
    let v=s.split(',').map(|v|v.parse::<f32>()).collect::<Result<Vec<_>,_>>().map_err(|_|"Three comma-separated numbers are required.")?;
    if v.len()!=3 || v.iter().any(|v|!v.is_finite()) {return Err("Three finite numbers are required.".into());}
    Ok(V::new(v[0],v[1],v[2]))
}
fn rgb(s:&str)->Result<V,String> {
    let c=triple(s)?;
    if [c.x,c.y,c.z].iter().any(|v|*v<0.0 || *v>255.0) {return Err("Color channels must be between 0 and 255.".into());}
    let linear=|v:f32| {let v=v/255.0; if v<=0.04045 {v/12.92} else {((v+0.055)/1.055).powf(2.4)}};
    Ok(V::new(linear(c.x),linear(c.y),linear(c.z)))
}
fn defaults()->Options {
    Options{model:None,lights:vec![],color:V::new(0.8,0.8,0.8),width:100,height:40,frames:None,mono:false,
        scale:1.0,zoom:1.0,position:V::default(),offset:(0.0,0.0),margins:[0;4],rotation:V::default(),
        speed:0.6,fps:30,distance:5.0,aspect:0.5,ambient:0.055,specular:0.35,shininess:32.0,
        chars:".,:;irsXA253hMHGS#9B&@".into(),auto_size:true,mtl:None,textures:true,shininess_override:false,
        pitch_limit:None,roll_limit:None}
}
fn options()->Result<Options,String> {
    let mut o=defaults();
    let mut args=env::args().skip(1);
    while let Some(arg)=args.next() {
        if arg=="--help" || arg=="-h" {
            println!("{HELP}");
            std::process::exit(0);
        }
        match arg.as_str() {
            "--once"=>o.frames=Some(1), "--mono"|"--monochrome"=>o.mono=true,
            "--static"=>o.speed=0.0,
            "--center"=>{o.position=V::default();o.offset=(0.0,0.0);o.auto_size=true;},
            "--auto-size"=>o.auto_size=true,
            "--fixed-size"=>o.auto_size=false,
            "--no-textures"=>o.textures=false,
            "--model"|"--mtl"|"--light"|"--color"|"--width"|"--height"|"--frames"|
            "--size"|"--scale"|"--zoom"|"--position"|"--offset"|"--margin"|"--margins"|
            "--rotation"|"--speed"|"--fps"|"--distance"|"--aspect"|
            "--pitch-limit"|"--roll-limit"|
            "--ambient"|"--specular"|"--shininess"|"--chars"=>{
                let value=args.next().ok_or_else(||format!("Missing value for {arg}"))?;
                match arg.as_str() {
                    "--model"=>o.model=Some(value),
                    "--mtl"=>o.mtl=Some(value),
                    "--light"=>{let (p,c)=value.split_once(':').ok_or("Light format: x,y,z:r,g,b")?; o.lights.push(Light{position:triple(p)?,color:rgb(c)?});}
                    "--color"=>o.color=rgb(&value)?,
                    "--position"=>o.position=triple(&value)?,
                    "--rotation"=>o.rotation=triple(&value)?,
                    "--pitch-limit"=>o.pitch_limit=Some(number(&value)?),
                    "--roll-limit"=>o.roll_limit=Some(number(&value)?),
                    "--offset"=>{let (x,y)=value.split_once(',').ok_or("Offset format: x,y")?;o.offset=(number(x)?,number(y)?);},
                    "--size"=>{let (w,h)=value.split_once('x').ok_or("Size format: 120x45")?;o.width=w.parse().map_err(|_|"An integer width is required")?;o.height=h.parse().map_err(|_|"An integer height is required")?;o.auto_size=false;},
                    "--margins"=>{let v=value.split(',').map(|n|n.parse::<usize>()).collect::<Result<Vec<_>,_>>().map_err(|_|"Margins must be nonnegative integers")?;o.margins=v.try_into().map_err(|_|"Four margins are required: left,top,right,bottom")?;},
                    "--chars"=>o.chars=value,
                    "--scale"=>o.scale=number(&value)?, "--zoom"=>o.zoom=number(&value)?,
                    "--speed"=>o.speed=number(&value)?, "--distance"=>o.distance=number(&value)?,
                    "--aspect"=>o.aspect=number(&value)?, "--ambient"=>o.ambient=number(&value)?,
                    "--specular"=>o.specular=number(&value)?, "--shininess"=>{o.shininess=number(&value)?;o.shininess_override=true;},
                    _=>{let n=value.parse::<usize>().map_err(|_|format!("An integer is required for {arg}"))?; match arg.as_str() {"--width"=>{o.width=n;o.auto_size=false;},"--height"=>{o.height=n;o.auto_size=false;},"--fps"=>o.fps=n,"--margin"=>o.margins=[n;4],_=>o.frames=Some(n)}}
                }
            }
            _=>return Err(format!("Unknown option: {arg}. Use --help.")),
        }
    }
    if !(10..=400).contains(&o.width) || !(5..=200).contains(&o.height) || o.frames==Some(0) {return Err("Width must be 10..400, height 5..200, and the frame count greater than zero.".into());}
    if o.margins.iter().any(|n|*n>400) || o.margins[0]+o.margins[2]+2>o.width || o.margins[1]+o.margins[3]+2>o.height {return Err("Margins must leave at least 2 columns and 2 rows for rendering.".into());}
    for (name,value,min,max) in [("scale",o.scale,0.01,10.0),("zoom",o.zoom,0.01,10.0),("distance",o.distance,0.2,100.0),("aspect",o.aspect,0.1,2.0),("speed",o.speed,-20.0,20.0),("ambient",o.ambient,0.0,2.0),("specular",o.specular,0.0,2.0),("shininess",o.shininess,1.0,256.0)] {
        if !(min..=max).contains(&value) {return Err(format!("--{name} must be between {min} and {max}."));}
    }
    if !(1..=120).contains(&o.fps) {return Err("--fps must be between 1 and 120.".into());}
    for (name,limit) in [("pitch-limit",o.pitch_limit),("roll-limit",o.roll_limit)] {
        if limit.is_some_and(|v|!(0.0..=180.0).contains(&v)) {return Err(format!("--{name} must be between 0 and 180 degrees (0 locks the axis)."));}
    }
    if [o.position.x,o.position.y,o.position.z,o.offset.0,o.offset.1].iter().any(|v|v.abs()>10000.0) {return Err("Position or offset is too large (maximum absolute value: 10000).".into());}
    if o.chars.len()<2 || o.chars.len()>128 || !o.chars.bytes().all(|b|(32..=126).contains(&b)) {return Err("--chars requires 2 to 128 printable ASCII characters, from dark to bright.".into());}
    if o.lights.is_empty() {o.lights=vec![
        Light{position:V::new(-3.0,3.0,-3.0),color:rgb("255,80,45")?},
        Light{position:V::new(3.0,1.0,-2.0),color:rgb("45,130,255")?},
        Light{position:V::new(0.0,-3.0,0.0),color:rgb("70,255,120")?}];}
    Ok(o)
}
fn shade(p:V,n:V,o:&Options,albedo:V,material:Option<&material::Material>)->V {
    let mut diffuse=V::new(o.ambient,o.ambient,o.ambient); let mut specular=V::default();
    let view=(V::new(0.0,0.0,-o.distance)-p).unit();
    let n=if n.dot(view)<0.0 {n * -1.0} else {n};
    for light in &o.lights {
        let delta=light.position-p; let l=delta.unit();
        let attenuation=2.0/(1.0+0.08*delta.dot(delta));
        let lambert=n.dot(l).max(0.0);
        diffuse=diffuse+light.color*(lambert*attenuation);
        let exponent=if o.shininess_override {o.shininess} else {material.and_then(|m|m.shininess).unwrap_or(o.shininess)};
        if lambert>0.0 {specular=specular+light.color*(n.dot((l+view).unit()).max(0.0).powf(exponent)*o.specular*attenuation);}
    }
    diffuse.product(albedo)+specular.product(material.map(|m|m.specular).unwrap_or(V::new(1.0,1.0,1.0)))
}
fn edge(a:V,b:V,p:V)->f32 {(p.x-a.x)*(b.y-a.y)-(p.y-a.y)*(b.x-a.x)}
// Clip triangles to the near plane before projection, including when moved behind the camera.
#[derive(Clone,Copy)]
struct Vertex {position:V,uv:V}
fn clip_near(triangle:[Vertex;3],distance:f32)->Vec<Vertex> {
    let plane=0.1-distance; let mut clipped=vec![];
    let mut previous=triangle[2];
    for current in triangle {
        let inside=current.position.z>=plane; let previous_inside=previous.position.z>=plane;
        if inside!=previous_inside {
            let t=(plane-previous.position.z)/(current.position.z-previous.position.z);
            clipped.push(Vertex{position:previous.position+(current.position-previous.position)*t,uv:previous.uv+(current.uv-previous.uv)*t});
        }
        if inside {clipped.push(current);} previous=current;
    } clipped
}
fn orientation(o:&Options,angle:f32)->V {
    let pitch=o.rotation.x+angle.to_degrees()*0.43;
    let roll=o.rotation.z;
    V::new(
        o.pitch_limit.map(|limit|pitch.clamp(-limit,limit)).unwrap_or(pitch),
        o.rotation.y+angle.to_degrees(),
        o.roll_limit.map(|limit|roll.clamp(-limit,limit)).unwrap_or(roll),
    )
}
fn render(m:&Mesh,o:&Options,angle:f32)->String {
    let mut depth=vec![0.0;o.width*o.height]; let mut colors=vec![None;o.width*o.height];
    let rotation=orientation(o,angle);
    let vertices:Vec<_>=m.vertices.iter().map(|p|p.euler(rotation)*o.scale+o.position).collect();
    // A live resize may make the requested margins larger than the window.
    let left=o.margins[0].min(o.width.saturating_sub(2));
    let top=o.margins[1].min(o.height.saturating_sub(2));
    let right=o.margins[2].min(o.width.saturating_sub(left+2));
    let bottom=o.margins[3].min(o.height.saturating_sub(top+2));
    let inner_width=o.width-left-right; let inner_height=o.height-top-bottom;
    let center_x=left as f32+inner_width as f32*0.5+o.offset.0;
    let center_y=top as f32+inner_height as f32*0.5+o.offset.1;
    let scale=(inner_width as f32*0.48).min(inner_height as f32*0.48/o.aspect)*3.0*o.zoom;
    for face in &m.faces {
        let world=face.indices.map(|i|vertices[i]);
        let normal=(world[1]-world[0]).cross(world[2]-world[0]).unit();
        let material=face.material.as_ref().and_then(|name|m.materials.get(name));
        let textured=o.textures && face.uv.iter().all(Option::is_some);
        let polygon=clip_near(std::array::from_fn(|i|Vertex{position:world[i],uv:face.uv[i].map(|u|m.uv[u]).unwrap_or_default()}),o.distance);
        for i in 1..polygon.len().saturating_sub(1) {
        let attributes=[polygon[0],polygon[i],polygon[i+1]];
        let world=attributes.map(|v|v.position);
        let screen=world.map(|p|V::new(center_x+p.x*scale/(p.z+o.distance),center_y-p.y*scale*o.aspect/(p.z+o.distance),1.0/(p.z+o.distance)));
        let area=edge(screen[0],screen[1],screen[2]); if area.abs()<1e-7 {continue;}
        let min_x=screen.iter().map(|v|v.x).fold(f32::INFINITY,f32::min).floor().max(left as f32) as usize;
        let max_x=screen.iter().map(|v|v.x).fold(f32::NEG_INFINITY,f32::max).ceil().max(0.0).min((o.width-right-1) as f32) as usize;
        let min_y=screen.iter().map(|v|v.y).fold(f32::INFINITY,f32::min).floor().max(top as f32) as usize;
        let max_y=screen.iter().map(|v|v.y).fold(f32::NEG_INFINITY,f32::max).ceil().max(0.0).min((o.height-bottom-1) as f32) as usize;
        if min_x>max_x || min_y>max_y {continue;}
        for y in min_y..=max_y {for x in min_x..=max_x {
            let p=V::new(x as f32+0.5,y as f32+0.5,0.0);
            let w=[edge(screen[1],screen[2],p)/area,edge(screen[2],screen[0],p)/area,edge(screen[0],screen[1],p)/area];
            if w.iter().any(|v|*v < -1e-5) {continue;}
            let inv_z=w[0]*screen[0].z+w[1]*screen[1].z+w[2]*screen[2].z; let index=y*o.width+x;
            if inv_z<=depth[index] {continue;}
            depth[index]=inv_z;
            // Perspective-correct world position for per-character lighting.
            let position=(world[0]*(w[0]*screen[0].z)+world[1]*(w[1]*screen[1].z)+world[2]*(w[2]*screen[2].z))*(1.0/inv_z);
            let uv=(attributes[0].uv*(w[0]*screen[0].z)+attributes[1].uv*(w[1]*screen[1].z)+attributes[2].uv*(w[2]*screen[2].z))*(1.0/inv_z);
            let texture=if textured {material.and_then(|m|m.texture.as_ref()).map(|t|t.sample(uv)).unwrap_or(V::new(1.0,1.0,1.0))} else {V::new(1.0,1.0,1.0)};
            let diffuse=material.map(|m|m.diffuse).unwrap_or(V::new(1.0,1.0,1.0));
            colors[index]=Some(shade(position,normal,o,o.color.product(diffuse).product(texture),material));
        }}
        }
    }
    let ramp=o.chars.as_bytes();
    let srgb=|v:f32| {let v=v.clamp(0.0,1.0); ((if v<=0.0031308 {v*12.92} else {1.055*v.powf(1.0/2.4)-0.055})*255.0).round() as u8};
    let mut result=String::with_capacity(o.width*o.height*20);
    for row in colors.chunks(o.width) {
        let mut previous=None;
        for pixel in row {
            if let Some(c)=pixel {
                let color=(srgb(c.x),srgb(c.y),srgb(c.z));
                if !o.mono && previous!=Some(color) {result.push_str(&format!("\x1b[38;2;{};{};{}m",color.0,color.1,color.2)); previous=Some(color);}
                let brightness=(0.2126*c.x+0.7152*c.y+0.0722*c.z).clamp(0.0,1.0).sqrt();
                result.push(ramp[(brightness*(ramp.len()-1) as f32) as usize] as char);
            } else {result.push(' ');}
        }
        if !o.mono {result.push_str("\x1b[0m");} result.push('\n');
    } result
}
#[cfg(windows)]
mod terminal {
    #[repr(C)] #[derive(Default)]
    struct Coord {x:i16,y:i16}
    #[repr(C)] #[derive(Default)]
    struct Rect {left:i16,top:i16,right:i16,bottom:i16}
    #[repr(C)] #[derive(Default)]
    struct Info {size:Coord,cursor:Coord,attributes:u16,window:Rect,maximum:Coord}
    #[link(name="kernel32")] unsafe extern "system" {
        fn GetStdHandle(kind:u32)->*mut std::ffi::c_void;
        fn GetConsoleMode(handle:*mut std::ffi::c_void,mode:*mut u32)->i32;
        fn SetConsoleMode(handle:*mut std::ffi::c_void,mode:u32)->i32;
        fn GetConsoleScreenBufferInfo(handle:*mut std::ffi::c_void,info:*mut Info)->i32;
    }
    #[link(name="msvcrt")] unsafe extern "C" {fn _kbhit()->i32; fn _getch()->i32;}
    pub fn enable() {unsafe {let h=GetStdHandle(-11i32 as u32); let mut mode=0; if GetConsoleMode(h,&mut mode)!=0 {SetConsoleMode(h,mode|4);}}}
    pub fn size()->Option<(usize,usize)> {
        unsafe {
            let mut info=Info::default();
            if GetConsoleScreenBufferInfo(GetStdHandle(-11i32 as u32),&mut info)==0 {return None;}
            let w=i32::from(info.window.right)-i32::from(info.window.left)+1;
            let h=i32::from(info.window.bottom)-i32::from(info.window.top)+1;
            if w>0 && h>0 {Some((w as usize,h as usize))} else {None}
        }
    }
    pub fn quit()->bool {unsafe {if _kbhit()!=0 {matches!(_getch(),27|81|113)} else {false}}}
}
#[cfg(not(windows))]
mod terminal {pub fn enable() {} pub fn quit()->bool {false} pub fn size()->Option<(usize,usize)> {None}}
fn resize(o:&mut Options,size:Option<(usize,usize)>)->bool {
    if !o.auto_size {return false;}
    if let Some((w,h))=size {
        let w=w.saturating_sub(1).clamp(1,400);
        let h=h.saturating_sub(1).clamp(1,200);
        if (o.width,o.height)!=(w,h) {o.width=w;o.height=h;return true;}
    }
    false
}
struct Screen;
impl Drop for Screen {fn drop(&mut self) {let _=write!(io::stdout(),"\x1b[0m\x1b[?25h\x1b[?1049l");}}
fn run()->Result<(),String> {
    let mut o=options()?;
    let m=if let Some(path)=&o.model {
        if o.mtl.is_some() {Mesh::load_with_mtl(path,o.mtl.as_deref())?} else {Mesh::load(path)?}
    } else {if o.mtl.is_some() {return Err("--mtl requires --model file.obj".into());}Mesh::torus()};
    terminal::enable();
    resize(&mut o,terminal::size());
    if o.frames==Some(1) {print!("{}",render(&m,&o,0.0)); return Ok(());}
    print!("\x1b[?1049h\x1b[2J\x1b[?25l"); let _screen=Screen;
    let start=Instant::now(); let mut frame=0; let mut out=io::stdout().lock();
    loop {
        let tick=Instant::now();
        if resize(&mut o,terminal::size()) {
            // Erase the old frame, including areas exposed when the window grows.
            write!(out,"\x1b[0m\x1b[2J").map_err(|e|e.to_string())?;
        }
        let image=render(&m,&o,start.elapsed().as_secs_f32()*o.speed);
        write!(out,"\x1b[H{}",image.trim_end_matches('\n')).map_err(|e|e.to_string())?;
        out.flush().map_err(|e|e.to_string())?; frame+=1;
        if o.frames.is_some_and(|limit|frame>=limit) || terminal::quit() {break;}
        thread::sleep(Duration::from_secs_f32(1.0/o.fps as f32).saturating_sub(tick.elapsed()));
    } Ok(())
}
fn main() {if let Err(error)=run() {eprintln!("Error: {error}"); std::process::exit(1);}}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pitch_and_roll_limits_bound_initial_rotation_and_animation() {
        let mut o=defaults();o.pitch_limit=Some(15.0);o.roll_limit=Some(5.0);
        o.rotation=V::new(35.0,20.0,-45.0);
        let initial=orientation(&o,0.0);
        assert_eq!(initial.x,15.0);assert_eq!(initial.y,20.0);assert_eq!(initial.z,-5.0);
        for angle in [-100.0,-1.0,0.0,1.0,100.0] {
            let r=orientation(&o,angle);
            assert!((-15.0..=15.0).contains(&r.x));assert!((-5.0..=5.0).contains(&r.z));
        }
        o.pitch_limit=Some(0.0);o.roll_limit=Some(0.0);
        for angle in [-2.0,0.0,2.0] {
            let r=orientation(&o,angle);
            assert_eq!(r.x,0.0);assert_eq!(r.z,0.0);
            let up=V::new(0.0,1.0,0.0).euler(r);
            assert!(up.x.abs()<1e-6 && (up.y-1.0).abs()<1e-6 && up.z.abs()<1e-6);
        }
        assert_ne!(orientation(&o,0.0).y,orientation(&o,1.0).y);
    }
    #[test]
    fn live_resize_keeps_model_centered_and_restores_margins() {
        let mut o=defaults(); o.mono=true; o.margins=[3;4];
        let m=Mesh::load("models/cube.obj").unwrap();
        for (w,h) in [(81,31),(121,46),(6,4),(1,1),(81,31)] {
            assert!(resize(&mut o,Some((w,h))));
            let image=render(&m,&o,0.0);
            let rows:Vec<_>=image.lines().collect();
            assert_eq!(rows.len(),o.height);
            assert!(rows.iter().all(|row|row.len()==o.width));
            if o.width>10 && o.height>10 {
                let mut points=vec![];
                for (y,row) in rows.iter().enumerate() {for (x,b) in row.bytes().enumerate() {
                    if b!=b' ' {points.push((x,y)); assert!(x>=3 && x<o.width-3 && y>=3 && y<o.height-3);}
                }}
                assert!(!points.is_empty());
                let min_x=points.iter().map(|p|p.0).min().unwrap();
                let max_x=points.iter().map(|p|p.0).max().unwrap();
                let min_y=points.iter().map(|p|p.1).min().unwrap();
                let max_y=points.iter().map(|p|p.1).max().unwrap();
                assert_eq!(min_x+max_x,o.width-1);
                assert_eq!(min_y+max_y,o.height-1);
            }
        }
        assert_eq!(o.margins,[3;4]);
        assert!(!resize(&mut o,Some((81,31))));
    }
    #[test]
    fn fixed_size_and_redirected_output_preserve_dimensions() {
        let mut o=defaults();
        assert!(!resize(&mut o,None));
        o.auto_size=false;
        assert!(!resize(&mut o,Some((160,60))));
        assert_eq!((o.width,o.height),(100,40));
    }
    #[test]
    fn mtl_texture_uv_orientation_and_rendered_colors() {
        let m=Mesh::load("models/textured-cube.obj").unwrap();
        assert_eq!(m.faces.len(),12);
        let mat=m.materials.get("checker").unwrap();
        let texture=mat.texture.as_ref().unwrap();
        let red=texture.sample(V::new(0.0,1.0,0.0));
        let blue=texture.sample(V::new(0.0,0.0,0.0));
        assert!(red.x>0.99 && red.y<0.01 && red.z<0.01);
        assert!(blue.z>0.99 && blue.x<0.01 && blue.y<0.01);
        let mut o=defaults();o.ambient=1.0;o.specular=0.0;
        let textured=render(&m,&o,0.0);
        o.textures=false;
        let plain=render(&m,&o,0.0);
        assert_ne!(textured,plain);
        o.mono=true;o.textures=true;
        let mono=render(&m,&o,0.0);
        assert!(!mono.contains('\x1b'));
        o.textures=false;
        assert_ne!(mono,render(&m,&o,0.0));
        // Clipping must retain finite, interpolated texture coordinates.
        o.scale=3.0;o.distance=1.0;o.textures=true;
        assert!(!render(&m,&o,0.3).is_empty());
    }
    #[test]
    fn negative_uv_indices_multiple_materials_and_mlt_extension() {
        let parent=Path::new("work/obj-material-test");fs::create_dir_all(parent).unwrap();
        fs::write(parent.join("custom.mlt"),"newmtl red\nKd 1 0 0\nnewmtl blue\nKd 0 0 1\n").unwrap();
        fs::write(parent.join("custom.obj"),"mtllib custom.mtl\nv -1 -1 0\nv 1 -1 0\nv 1 1 0\nv -1 1 0\nvt 0 0\nvt 1 0\nvt 1 1\nvt 0 1\nusemtl red\nf -4/-4 -3/-3 -2/-2\nusemtl blue\nf -4/-4 -2/-2 -1/-1\n").unwrap();
        let m=Mesh::load(parent.join("custom.obj").to_str().unwrap()).unwrap();
        assert_eq!(m.materials.len(),2);
        assert_eq!(m.faces[0].uv,[Some(0),Some(1),Some(2)]);
        assert_eq!(m.faces[1].material.as_deref(),Some("blue"));
        let mut o=defaults();o.ambient=1.0;o.specular=0.0;
        let rendered=render(&m,&o,0.0);
        assert!(rendered.contains("[38;2;231;0;0m"));
        assert!(rendered.contains("[38;2;0;0;231m"));
        assert!(Mesh::load_with_mtl(parent.join("custom.obj").to_str().unwrap(),Some("absent.mtl")).is_err());
    }
}
