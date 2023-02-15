use std::cmp::min;
use std::collections::BTreeMap;

use crate::object::*;
use crate::text::*;

pub fn attributes(m: &Vec<u8>, v: u8, o: usize) -> String {
    let count = match v {
        1 | 2 | 3 => 32,
        4 | 5 | 6 | 7 | 8 => 48,
        _ => 0,
    };

    let mut s = String::new();
    for i in 0..count {
        if attribute(m, v, o, i) == 1 {
            s.push_str(&format!("{} ", i));
        }
    }

    s.trim().to_string()
}

pub fn print_object(m: &Vec<u8>, v: u8, o: usize) {
    let pta = property_table_address(m, v, o);

    println!("Object #{}:", o);
    println!("\tAttributes: {}", attributes(m, v, o));
    println!(
        "\tParent: {:5} Sibling: {:5} Child {:5}",
        parent(m, v, o),
        sibling(m, v, o),
        child(m, v, o)
    );
    println!("\tProperty table: ${:04x}", pta);
    println!("\t\tDescription: \"{}\"", as_text(m, v, pta + 1));
    println!("\t\t Properties:");

    let prop_cnt = match v {
        1 | 2 | 3 => 31,
        4 | 5 | 6 | 7 | 8 => 63,
        _ => 0,
    };

    for i in (1..=prop_cnt).rev() {
        if has_property(m, v, o, i) {
            let d = property(m, v, o, i);
            print!("\t\t\t[{:2}]", i);
            for b in d {
                print!(" {:02x}", b);
            }
            println!("");
        }
    }
}

pub fn print_object_table(m: &Vec<u8>, v: u8) {
    let mut min_prop_table: usize = 0xFFFF;
    let max_obj = match v {
        1 | 2 | 3 => 255,
        4 | 5 | 6 | 7 | 8 => 65535,
        _ => 0,
    };

    let mut i = 1;
    while object_address(m, v, i + 1) < min_prop_table && i <= max_obj {
        print_object(m, v, i);
        i = i + 1;
        min_prop_table = min(min_prop_table, property_table_address(m, v, i));
    }
}

fn print_branch(m: &Vec<u8>, v: u8, tree: &BTreeMap<usize, Vec<usize>>, o: usize, d: usize) {
    for _i in 0..d {
        match v {
            1 | 2 | 3 => print!(" . "),
            4 | 5 | 6 | 7 | 8 => print!("  .  "),
            _ => {}
        }
    }

    match v {
        1 | 2 | 3 => println!("[{:3}] \"{}\"", o, from_vec(m, v, &short_name(m, v, o))),
        4 | 5 | 6 | 7 | 8 => println!("[{:5}] \"{}\"", o, from_vec(m, v, &short_name(m, v, o))),
        _ => {}
    }

    if tree.contains_key(&o) {
        for k in tree.get(&o).unwrap() {
            print_branch(m, v, tree, *k, d + 1);
        }
    }
}
pub fn print_object_tree(m: &Vec<u8>, v: u8) {
    let mut min_prop_table: usize = 0xFFFF;
    let max_obj = match v {
        1 | 2 | 3 => 255,
        4 | 5 | 6 | 7 | 8 => 65535,
        _ => 0,
    };

    let mut i = 1;
    let mut tree: BTreeMap<usize, Vec<usize>> = BTreeMap::new();

    while object_address(m, v, i + 1) < min_prop_table && i <= max_obj {
        let p = parent(m, v, i);
        if p == 0 {
            tree.insert(i, Vec::new());
        } else {
            match tree.get_mut(&p) {
                Some(e) => {
                    e.push(i);
                }
                None => {
                    let mut v = Vec::new();
                    v.push(i);
                    tree.insert(p, v);
                }
            }
        }

        i = i + 1;
        min_prop_table = min(min_prop_table, property_table_address(m, v, i));
    }

    for k in tree.keys() {
        if parent(m, v, *k) == 0 {
            print_branch(m, v, &tree, *k, 0);
        }
    }
}

pub fn print_default_properties(m: &Vec<u8>, v: u8) {
    let prop_cnt = match v {
        1 | 2 | 3 => 31,
        4 | 5 | 6 | 7 | 8 => 63,
        _ => 0
    };

    println!("Default properties:");
    for i in 1..=prop_cnt {
        print!("\t[{:2}]", i);
        for b in default_property(m, i) {
            print!(" {:02x}", b);
        }
        println!("");
    }
}