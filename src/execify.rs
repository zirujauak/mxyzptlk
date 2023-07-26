use std::{
    env,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    process::exit,
};

use iff::Chunk;

#[macro_use]
extern crate log;

fn main() {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let full_name = filename.split('.').collect::<Vec<&str>>()[0].to_string();
    let mut zcode = Vec::new();

    match File::open(filename) {
        Ok(mut f) => match f.read_to_end(&mut zcode) {
            Ok(_) => {}
            Err(e) => {
                error!(target: "app::trace", "Error reading {}: {}", filename, e);
                println!("Error reading {}", filename);
                exit(-1);
            }
        },
        Err(e) => {
            error!(target: "app::trace", "Error reading {}: {}", filename, e);
            println!("Error reading {}", filename);
            exit(-1);
        }
    }

    let blorb = full_name.to_string() + ".blorb";
    let iff = match File::open(&blorb) {
        Ok(mut f) => match Chunk::try_from(&mut f) {
            Ok(i) => i,
            Err(e) => {
                println!("Error reading blorb {}: {}", blorb, e);
                exit(-1);
            }
        },
        Err(e) => {
            println!("Error opening blorb {}: {}", blorb, e);
            exit(-1);
        }
    };

    if let Some(ridx) = iff.find_chunk("RIdx", "").as_mut() {
        let mut data = ridx.data().clone();
        let count = iff::vec_as_unsigned(&ridx.data()[0..4]);
        // Increment the count by 1
        let nc = iff::unsigned_as_vec(count + 1, 4);
        data[0] = nc[0];
        data[1] = nc[1];
        data[2] = nc[2];
        data[3] = nc[3];
        // We'll be adding 12 bytes to this chunk, so adjust the offsets of
        // all indices by 12
        for i in 0..count {
            let offset = 12 + (i * 12);
            let start = iff::vec_as_unsigned(&ridx.data()[offset..offset + 4]);
            let new_start = start + 12;
            let ns = iff::unsigned_as_vec(new_start, 4);
            data[offset] = ns[0];
            data[offset + 1] = ns[1];
            data[offset + 2] = ns[2];
            data[offset + 3] = ns[3];
        }
        // The new chunk will be placed at the end of the current file + 8 bytes IFF header + 12 bytes index entry
        let start = iff::unsigned_as_vec(iff.length() as usize + 20, 4);
        // Add a new index for the 'Exec' chunk
        data.extend(&vec![
            b'E', b'x', b'e', b'c', 0x00, 0x00, 0x00, 0x00, start[0], start[1], start[2], start[3],
        ]);

        let new_ridx = Chunk::new_chunk(ridx.offset(), "RIdx", data);
        let exec = Chunk::new_chunk(iff.length() + 20, "ZCOD", zcode);

        let mut chunks = Vec::new();
        // Push the new RIdx chunk
        chunks.push(new_ridx);
        // And everything else
        for c in iff.chunks() {
            if c.id() != "RIdx" {
                chunks.push(c.clone());
            }
        }
        // And finally the ZCODE chunk
        chunks.push(exec);
        let new_iff = Chunk::new_form(0, "IFRS", chunks);
        let new_filename = full_name + "-new.blorb";
        if Path::new(&new_filename).exists() {
            println!("Destination {} already exists.", new_filename);
            exit(-1);
        }

        match fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&new_filename)
        {
            Ok(mut f) => {
                f.write_all(&Vec::from(&new_iff));
                f.flush();
            }
            Err(e) => {
                println!("Error opening new file {}: {}", new_filename, e);
                exit(-1);
            }
        }

        println!("Write to {} complete", new_filename);
    }
}
