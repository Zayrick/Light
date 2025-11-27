use std::sync::OnceLock;
use std::fs::File;
use std::io::Read;

const LUT_DIM: usize = 256;
const LUT_CHANNELS: usize = 3;
const LUT_SIZE: usize = LUT_DIM * LUT_DIM * LUT_DIM * LUT_CHANNELS;

static HDR_LUT: OnceLock<Option<Vec<u8>>> = OnceLock::new();

pub fn get_hdr_lut() -> Option<&'static [u8]> {
    HDR_LUT.get_or_init(|| {
        load_lut_from_file()
    }).as_deref()
}

fn load_lut_from_file() -> Option<Vec<u8>> {
    // Try different paths to find the LUT file
    let paths = [
        "src/resource/lut/lut_lin_tables.3d",
        "resource/lut/lut_lin_tables.3d", 
        "../src/resource/lut/lut_lin_tables.3d",
    ];

    for p in paths {
        if let Ok(mut file) = File::open(p) {
            // We only need the first LUT (HDR RGB), which is the first LUT_SIZE bytes.
            let mut buffer = vec![0u8; LUT_SIZE];
            if file.read_exact(&mut buffer).is_ok() {
                println!("[LUT] Loaded HDR LUT from {}", p);
                return Some(buffer);
            }
        }
    }
    
    eprintln!("[LUT] Could not find or read lut_lin_tables.3d");
    None
}

#[inline(always)]
pub fn apply_lut(r: u8, g: u8, b: u8, lut: &[u8]) -> (u8, u8, u8) {
    // LUT_INDEX(y,u,v) ((y + (u<<8) + (v<<16))*3)
    // y=R, u=G, v=B
    let index = ((r as usize) + ((g as usize) << 8) + ((b as usize) << 16)) * 3;
    
    // Unsafe get for performance? 
    // Since we allocated exactly LUT_SIZE (256^3 * 3), and r,g,b are u8,
    // max index is (255 + 255*256 + 255*65536)*3 = 16777215*3 = 50331645.
    // LUT_SIZE is 50331648.
    // So index+2 is max 50331647.
    // It is safe.
    
    if index + 2 < lut.len() {
        (lut[index], lut[index+1], lut[index+2])
    } else {
        (r, g, b)
    }
}

