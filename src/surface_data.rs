use bytemuck:: {Pod, Zeroable};
use srtm::Tile;
use std::thread;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::mem;
//mod colormap;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}
trait Defaultable {
    fn default_with_params(lat: u32,long:u32) -> Self;
}
struct Threaded {
    pub refer: std::sync::mpsc::Receiver<Vec<Vec<f32>>>,
    pub thread:JoinHandle<()>,
}
impl Defaultable for Threaded {
    fn default_with_params(lat:u32,long:u32) -> Self {
        let (tx, rx) = mpsc::channel();
        let thread = thread::spawn(move||{
            let mut map :Vec<Vec<f32>> = vec![];
            let mut height_min = f32::MAX;
            let mut height_max = f32::MIN;
            let worldmap: Tile = Tile::from_file("src/Scotlandhgt/N".to_owned() + &*lat.to_string() +"W00"+ &*long.to_string() +".hgt").unwrap_or(Tile::from_file("src/Scotlandhgt/N00W000.hgt").unwrap());

            for x in 0..3600{
                let mut p1:Vec<f32> = vec![];
                for z in 0..3600{
                    let y =  Tile::get(&worldmap, x as u32, z as u32) as f32;
                    height_min = if y  < height_min { y } else { height_min };
                    height_max = if y  > height_max { y } else { height_max };
                    p1.push(y);

                }
                map.push(p1);
            }

            for x in 0..3600 as usize {
                for z in 0..3600 as usize {
                    map[x][z] = (map[x][z] as f32 - height_min)/(height_max - height_min);
                }
            }
            tx.send(map).unwrap();
        });
        Self {
            refer: rx,
            thread: thread,
        }
    }
}

impl Threaded {
    fn transfer(&mut self, lat:u32,long:u32){
        let (tx, rx) = mpsc::channel();
        let thread = thread::spawn(move||{
            let mut map :Vec<Vec<f32>> = vec![];
            let mut height_min = f32::MAX;
            let mut height_max = f32::MIN;
            let worldmap: Tile = Tile::from_file("src/Scotlandhgt/N".to_owned() + &*lat.to_string() +"W00"+ &*long.to_string() +".hgt").unwrap_or(Tile::from_file("src/Scotlandhgt/N00W000.hgt").unwrap());

            for x in 0..3600{
                let mut p1:Vec<f32> = vec![];
                for z in 0..3600{
                    let y =  Tile::get(&worldmap, x as u32, z as u32) as f32;
                    height_min = if y  < height_min { y } else { height_min };
                    height_max = if y  > height_max { y } else { height_max };
                    p1.push(y);

                }
                map.push(p1);
            }

            for x in 0..3600 as usize {
                for z in 0..3600 as usize {
                    map[x][z] = (map[x][z] as f32 - height_min)/(height_max - height_min);
                }
            }
            tx.send(map).unwrap();
        });
        self.thread=thread;
        self.refer=rx;
    }
    fn transferwithret(&mut self, lat:u32,long:u32){
        let tryy = Threaded::default_with_params(lat,long);
        self.thread=tryy.thread;
        self.refer=tryy.refer;
    }
}


pub struct Terrain {
    pub offsets: [f32; 2],
    pub moves: [f32; 2],
    pub back: [i32; 2],
    pub level_of_detail: u32,
    pub water_level: f32,
    pub mapdata: Vec<Vec<f32>>,
    pub mapdata2: Vec<Vec<f32>>,
    pub done1 :u32,
    pub done2 :u32,
    pub lat :u32,
    pub long :u32,
    pub chunksize:u32,
    nthread: Threaded,
    ethread: Threaded,
    sthread: Threaded,
    wthread: Threaded,

}

impl Default for Terrain {
        fn default() -> Self {
            let mut lat =54;
            let mut long = 4;
            let norththread = Threaded::default_with_params(lat,long);
            let eastthread = Threaded::default_with_params(lat,long);
            long -=2;
            let souththread = Threaded::default_with_params(lat,long);
            let westthread = Threaded::default_with_params(lat,long);
        Self {
            offsets: [0.0, 0.0],
            moves:[1800.0,1800.0],
            back:[0,0],
            level_of_detail: 5,
            water_level: 0.1,
            mapdata: vec![],
            mapdata2: vec![],
            done1:0,
            done2:0,
            chunksize:241,
            lat:54,
            long:3,
            nthread: norththread,
            ethread: eastthread,
            sthread: souththread,
            wthread: westthread,
        }
    }
}

impl Terrain {

    pub fn create_indices(&mut self, width: u32, height: u32) -> Vec<u32> {
        let n_vertices_per_row = height;
        let mut indices:Vec<u32> = vec![];

        for i in 0..width - 1 {
            for j in 0..height - 1 {
                let idx0 = j + i * n_vertices_per_row;
                let idx1 = j + 1 + i * n_vertices_per_row;
                let idx2 = j + 1 + (i + 1) * n_vertices_per_row;
                let idx3 = j + (i + 1) * n_vertices_per_row;
                indices.extend([idx0, idx1, idx2, idx2, idx3, idx0]);
            }
        }
        indices
    }
    pub fn find_world_map(&mut self) {
        let mut height_min = f32::MAX;
        let mut height_max = f32::MIN;
        let worldmap: Tile =  Tile::from_file("src/Scotlandhgt/N".to_owned() + &*self.lat.to_string() +"W00"+ &*self.long.to_string() +".hgt").unwrap_or(Tile::from_file("src/Scotlandhgt/N00W000.hgt").unwrap());

        for x in 0..3600 {
            let mut p1:Vec<f32> = vec![];
            for z in 0..3600 {
                let y =  Tile::get(&worldmap, x as u32, z as u32) as f32;
                height_min = if y < height_min { y } else { height_min };
                height_max = if y > height_max { y } else { height_max };
                p1.push(y);
            }
            self.mapdata.push(p1);
        }

        for x in 0..3600 as usize {
            for z in 0..3600 as usize {
                self.mapdata[x][z] = (self.mapdata[x][z] - height_min)/(height_max - height_min);
            }
        }
    }
    fn color_interp(&mut self, color:&Vec<[f32;3]>, ta:&Vec<f32>, t:f32) -> [f32;3] {
        let len = 6usize;
        let mut res = [0f32;3];
        for i in 0..len - 1 {
            if t >= ta[i] && t < ta[i + 1] {
                res = color[i];
            }
        }
        if t == ta[len-1] {
            res = color[len-2];
        }
        res
    }

    fn add_terrain_colors(&mut self, color:&Vec<[f32;3]>, ta:&Vec<f32>, tmin:f32, tmax:f32, t:f32) -> [f32;3] {
        let mut tt = if t < tmin { tmin } else if t > tmax { tmax } else { t };
        tt = (tt - tmin)/(tmax - tmin);
        let t1 = self.shift_water_level(ta);
        self.color_interp(color, &t1, tt)
    }
    fn shift_water_level(&mut self, ta:&Vec<f32>) -> Vec<f32> {
        let mut t1 = vec![0f32; 6];
        let r = (1.0 - self.water_level)/(1.0 - ta[1]);
        t1[1] = self.water_level;
        for i in 1..5usize {
            let del = ta[i+1] - ta[i];
            t1[i+1] = t1[i] + r * del;
        }
        t1
    }



    pub fn create_terrain_data(&mut self) -> (Vec<Vertex>, u32) {

        let increment_count = if self.level_of_detail <= 5 { self.level_of_detail + 1} else { 2*(self.level_of_detail - 2)};
        let vertices_per_row = (self.chunksize - 1)/increment_count + 1;
        let cdata =  vec![
            [0.055f32, 0.529, 0.8],
            [0.761, 0.698, 0.502],
            [0.204, 0.549, 0.192],
            [0.353, 0.302, 0.255],
            [1.0, 0.98, 0.98]
        ];
        let ta = vec![0.0f32, 0.3, 0.35, 0.6, 0.9, 1.0];

        if self.done1 == 0 {
            self.find_world_map();
            self.done1 +=1
        }


        let mut data:Vec<Vertex> = vec![];

        for x in (0..self.chunksize as usize).step_by(increment_count as usize) {
            for z in (0..self.chunksize as usize).step_by(increment_count as usize) {
                let usex : i32= (x as f32 + self.offsets[0] + self.moves[0])as i32;
                let usez : i32= (z as f32 + self.offsets[1] + self.moves[1])as i32;
                //let usez = z as f32 + self.offsets[1] + self.moves[1];
                let mut y = 0.0;
                if (usez > 200 && usez <= 3400)&&(usex > 200 && usex <= 3400){
                    y = self.mapdata[usex as usize][usez as usize];
                    if self.done1 ==2 {
                            Threaded::transferwithret(&mut self.nthread,self.lat,self.long+1);
                            Threaded::transferwithret(&mut self.sthread,self.lat,self.long-1);
                            Threaded::transferwithret(&mut self.ethread,self.lat+1,self.long);
                            Threaded::transferwithret(&mut self.wthread,self.lat-1,self.long);
                            self.done1 +=1;
                         }
                }
                if (usez <= 200 && usez >=0)||(usex <= 200 && usex >=0){
                    if usez <= 200 && usez >=0{
                        if self.done2 == 0 {
                        for received in &self.sthread.refer {
                            self.mapdata2 = vec![];
                            self.mapdata2 = received
                        }
                        self.done2 +=1;
                        }
                         y = self.mapdata[usex as usize][usez as usize];
                    }
                    if usex <= 200 && usex >=0{
                        if self.done2 == 0 {
                        for received in &self.wthread.refer {
                            self.mapdata2 = vec![];
                            self.mapdata2 = received
                        }
                        self.done2 +=1;
                        }
                         y = self.mapdata[usex as usize][(usez as i32 +self.back[1]) as usize];
                    }
                }
                if (usez < 0 && usez >= -1800)||(usex < 0 && usex >= -1800){
                    if usez < 0 && usez >= -1800 {
                        self.back[1] = 3600;
                        y = self.mapdata2[usex as usize][(usez as i32 + self.back[1]) as usize];
                    }
                    if usex < 0 && usex >= -1800 {

                    }
                }
                if (usez < -1800)&&(usex < -1800){
                    y = self.mapdata[usex as usize][usez as usize];
                    if self.done1 ==2 {
                            Threaded::transferwithret(&mut self.nthread,self.lat,self.long+1);
                            Threaded::transferwithret(&mut self.sthread,self.lat,self.long-1);
                            Threaded::transferwithret(&mut self.ethread,self.lat+1,self.long);
                            Threaded::transferwithret(&mut self.wthread,self.lat-1,self.long);
                            self.done1 +=1;
                         }
                }
                match usez as i32 {
                    m if m < -1800 =>{
                            self.moves[1] += 3600.0;
                            self.back[1] = 0;
                            self.mapdata = vec![];
                            self.mapdata = self.mapdata2.clone();
                            self.long -= 1;
                            self.done1 = 2;
                            self.done2 = 0;
                    }
                    -1800 ..=-1 => {
                        self.back[1] = 3600;
                        y = self.mapdata2[usex as usize][(usez as i32 +self.back[1]) as usize];

                    }
                     0 ..=200 => {
                        if self.done2 == 0 {
                        for received in &self.sthread.refer {
                            self.mapdata2 = vec![];
                            self.mapdata2 = received
                        }
                        self.done2 +=1;
                        }
                         y = self.mapdata[usex as usize][usez as usize];
                    }
                    201 ..=3400 => {

                        y = self.mapdata[usex as usize][usez as usize];
                        if self.done1 ==2 {
                            Threaded::transferwithret(&mut self.nthread,self.lat,self.long+1);
                            Threaded::transferwithret(&mut self.sthread,self.lat,self.long-1);
                            self.done1 +=1;
                         }
                    }
                    3401 ..= 3599=>{
                        if self.done2 == 0 {
                        for received in &self.nthread.refer {
                            self.mapdata2 = vec![];
                            self.mapdata2 = received
                        }
                        self.done2 +=1;
                        }
                        y = self.mapdata[usex as usize][usez as usize];
                    }
                    3600 ..= 5000=>{
                        self.back[1] = -3600;
                        y = self.mapdata2[usex as usize][(usez as i32 + self.back[1])as usize];
                    }
                    _ => {
                        self.moves[1] -=3600.0;
                        self.back[1] = 0;
                        self.mapdata = vec![];
                        self.mapdata = self.mapdata2.clone();
                        self.long +=1;
                        self.done1=2;
                        self.done2=0;

                }}/*
                match usex as i32 {
                    n if n < -1400 =>{
                        //self.moves[0] += 3600.0;
                        //self.back[0] = 0;
                        self.mapdata = vec![];
                        self.mapdata = self.mapdata2.clone();
                        self.lat -=1;
                        self.done1=2;
                        self.done2=0;
                    }
                    -1400 ..=-1 => {
                        self.back[0] += 3600;
                        y = self.mapdata2[usex as usize][(usez as i32 +self.back[1]) as usize];


                    }
                     0 ..=200 => {
                        if self.done2 == 0 {
                        for received in &self.wthread.refer {
                            self.mapdata2 = vec![];
                            self.mapdata2 = received
                        }
                        self.done2 +=1;
                        }
                         y = self.mapdata[usex as usize][(usez as i32 +self.back[1]) as usize];
                    }
                    201 ..=3400 => {
                        y = self.mapdata[usex as usize][(usez as i32 +self.back[1]) as usize];
                        if self.done1 ==2 {
                            Threaded::transferwithret(&mut self.ethread,self.lat+1,self.long);
                            Threaded::transferwithret(&mut self.wthread,self.lat-1,self.long);
                            self.done1 +=1;
                         }
                    }
                    3401 ..= 3599=>{
                        if self.done2 == 0 {
                        for received in &self.ethread.refer {
                            self.mapdata2 = vec![];
                            self.mapdata2 = received
                        }
                        self.done2 +=1;
                        }
                        y = self.mapdata[usex as usize][(usez as i32 +self.back[1]) as usize];
                    }
                    3600 ..= 5000=>{
                        self.back[0] -= 3600;
                        y = self.mapdata2[usex as usize][(usez as i32 +self.back[1]) as usize];
                    }
                    _ => {
                        self.moves[0] -=3600.0;
                        self.back[0] = 0;
                        self.mapdata = vec![];
                        self.mapdata = self.mapdata2.clone();
                        self.lat +=1;
                        self.done1=2;
                        self.done2=0;

                }}*/

                if y < self.water_level {
                    y = self.water_level - 0.01;
                }
                let position = [x as f32, y, z as f32];
                let color = self.add_terrain_colors(&cdata, &ta, 0.0, 1.0, y);

                data.push(Vertex { position, color });
            }
        }
    (data,vertices_per_row)
    }
    pub fn create_collection_of_terrain_data(&mut self, x_chunks:u32, z_chunks:u32, translations:&Vec<[f32;2]>) -> (Vec<Vec<Vertex>>, u32) {
        let mut data:Vec<Vec<Vertex>> = vec![];
        let mut vertices_per_row = 0u32;

        let mut k:u32 = 0;
        for _i in 0..x_chunks {
            //self.level_of_detail=5;
            for _j in 0..z_chunks {
                self.offsets = translations[k as usize];
                let dd = self.create_terrain_data();
                data.push(dd.0);
                vertices_per_row = dd.1;
                k += 1;
            }
        }
        (data, vertices_per_row)
    }





}
