use std::fmt;

use crate::text::as_text;

pub struct Property {
    address: usize,
    number: u8,
    size: usize,
    data: Vec<u8>
}

impl fmt::Display for Property {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\tNumber: {}", self.number)?;
        writeln!(f, "\tSize: {}", self.size)?;
        write!(f, "\tData: [")?;
        for b in &self.data {
            write!(f, " ${:02x}", b)?;
        }
        write!(f, " ]")
    }
}
impl Property {
    fn from_address(m: &Vec<u8>, v: u8, a: usize) -> Property {
        match v {
            1 | 2 | 3 => {
                let size = (m[a] / 32) as usize + 1;
                let num = m[a] & 0x1F;
                let data = m.as_slice()[a+1..a+1+size].to_vec();
                Property {
                    address: a,
                    number: num,
                    size: size,
                    data: data
                }
            },
            _ => Property {
                address: 0,
                number: 0,
                size: 0,
                data: Vec::new()
            }
        }
    }
}

pub struct Object {
    address: usize,
    attributes: Vec<u8>,
    parent: usize,
    sibling: usize,
    child: usize,
    property_table_address: usize,
    short_name: String,
    properties: Vec<Property>
}

impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\tAddress: ${:x}", self.address)?;
        for i in 0..self.attributes.len() {
            writeln!(f, "\tAttribute {}: {}", i, self.attributes[i])?;
        }
        writeln!(f, "\tParent: {}", self.parent)?;
        writeln!(f, "\tSibling: {}", self.sibling)?;
        writeln!(f, "\tChild: {}", self.child)?;
        writeln!(f, "\tProperty table: ${:x}", self.property_table_address)?;
        writeln!(f, "\tShort name: {}", self.short_name)?;
        writeln!(f, "\tProperties:")?;
        for p in &self.properties {
            writeln!(f, "{}", p)?;
        }
        Ok(())
    }
}

impl Object {
    fn from_addr(m: &Vec<u8>, v: u8, a: usize) -> Object {
        let mut attributes = Vec::new();
        let mut parent = 0;
        let mut sibling = 0;
        let mut child = 0;
        let mut prop_addr = 0;
        let mut props = Vec::new();
        let mut short_name = String::new();
        
        match v {
            1 | 2 | 3 => {
                /* Attributes are stored as:
                   0:  byte 0, bit 7
                   7:  byte 0, bit 0
                   8:  byte 1, bit 7
                   15: byte 1, bit 0
                   16: byte 2, bit 7
                   23: byte 2, bit 0
                   24: byte 3, bit 0
                   31: byte 3, bit 7 */
                for i in (1..=4).rev() {
                    for j in (1..=8).rev() {
                        let b = m[a + 4 - i];
                        let attr = b >> (j - 1) & 0x1;
                        attributes.push(attr);
                    }
                }
                parent = m[a+4];
                sibling = m[a+5];
                child = m[a+6];
                prop_addr = word_value(m, a + 7) as usize;
                let ph_s = m[prop_addr] as usize;
                short_name = as_text(m, prop_addr + 1);

                let mut go = true;
                let mut pa = prop_addr + (ph_s * 2) + 1;
                while go {
                    let p = Property::from_address(m, v, pa);
                    if p.number > 0 {
                        pa = pa + p.size + 1;
                        props.push(p);
                    } else {
                        go = false;
                    }
                }
                
            },
            _ => {}
        }
        Object {
            address: a,
            attributes: attributes,
            parent: parent as usize,
            sibling: sibling as usize,
            child: child as usize,
            property_table_address: prop_addr,
            short_name: short_name,
            properties: props,
        }
    }
}

pub struct ObjectTable {
    version: u8,
    property_defaults: Vec<u16>,
    objects: Vec<Object>
}

impl fmt::Display for ObjectTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Default properties:")?;
        for i in 0..self.property_defaults.len() {
            writeln!(f, "\t{}: ${:x}", i + 1, self.property_defaults[i])?;
        }
        writeln!(f, "Objects:")?;
        for i in 0..self.objects.len() {
            writeln!(f, "\t{}\n{}", i + 1, self.objects[i])?;
        }

        Ok(())
    }
}

fn word_value(v: &Vec<u8>, a: usize) -> u16 {
    let hb: u16 = (((v[a] as u16) << 8) as u16 & 0xFF00) as u16;
    let lb: u16 = (v[a + 1] & 0xFF) as u16;
    hb + lb
}

impl ObjectTable {
    pub fn from_addr(m: &Vec<u8>, a: usize) -> ObjectTable {
        let v = m[0];
        let mut prop_def = Vec::new();
        let prop_cnt = match v {
            1 | 2 | 3 => 32,
            _ => 64
        };

        for i in 1..prop_cnt {
            prop_def.push(word_value(m, a + (i * 2)))
        }

        let mut o = Vec::new();
        for i in 0..5 {
            o.push(Object::from_addr(m, v, a + (i * 9) + ((prop_cnt-1) * 2)));
        }
        ObjectTable {
            version: v,
            property_defaults: prop_def,
            objects: o
        }
    }
}