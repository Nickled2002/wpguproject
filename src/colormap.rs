#![allow(dead_code)]
///NOT USED ANYMORE MOVED TO SURFACE DATA
pub fn color_interp(colors:[[f32;3];11], min:f32, max:f32, mut t:f32) -> [f32; 3]{
    //used to determine the color based on the y value
    if t < min {
        t = min;
    }
    if t > max {
        t = max;

    }
    let tn = (t-min)/(max - min);
    let index = (10.0 * tn).floor() as usize;

    if index as f32 == 10.0 * tn {
        colors[index]
    } else {
        let tn1 = (tn - 0.1 * index as f32)*10.0; // rescale
        let a = colors[index];
        let b = colors[index +1];
        let color_r = a[0] + (b[0] - a[0]) * tn1;
        let color_g = a[1] + (b[1] - a[1]) * tn1;
        let color_b = a[2] + (b[2] - a[2]) * tn1;
        [color_r, color_g, color_b]
    }
}

pub fn colormap_data(colormap_name: &str) -> [[f32; 3]; 11] {
    let colors = match colormap_name {//pre configed color maps
        "mountain" => [[0.0,0.0,1.0],[0.7,0.7,0.5],[0.0,1.0,0.0],[0.0,1.0,0.0],[0.25,0.16,0.1],[0.25,0.16,0.1],
            [0.25,0.16,0.1],[0.25,0.16,0.1],[0.25,0.16,0.1],[1.0,1.0,1.0],[1.0,1.0,1.0]],
        "test" =>[[0.0000,0.4627,0.0275],[0.0000,0.3216,0.1176],[0.0000,0.1686,0.2196],[0.0000,0.0392,0.3098],
            [0.0000,0.0902,0.3961],[0.0000,0.2275,0.4863],[0.0000,0.3804,0.5843],[0.0510,0.5255,0.6863],
            [0.3137,0.6549,0.7686],[0.5922,0.7961,0.8627],[0.9020,0.9490,0.9647]],
        // "jet" as default
        _ => [[0.0,0.0,0.51],[0.0,0.24,0.67],[0.01,0.49,0.78],[0.01,0.75,0.89],[0.02,1.0,1.0],
            [0.51,1.0,0.5],[1.0,1.0,0.0],[0.99,0.67,0.0],[0.99,0.33,0.0],[0.98,0.0,0.0],[0.5,0.0,0.0]],
    };

    colors
}