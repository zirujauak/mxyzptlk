/// Read a word from a memory map
///
/// # Arguments:
///
/// * `m` - Memory map
/// * `a` - address
fn word_value(m: &Vec<u8>, a: usize) -> u16 {
    let hb = m[a];
    let lb = m[a + 1];

    (((hb as u16) << 8) & 0xFF00) + ((lb as u16) & 0xFF)
}

pub struct Frame {
    pc: usize,
    stack: Vec<u16>,
    local_variables: Vec<u16>,
    result_var: Option<u8>
}

pub fn initial_frame(a: usize) -> Frame {
    Frame {
        pc: a,
        stack: Vec::new(),
        local_variables: Vec::new(),
        result_var: None
    }
}
pub fn new_frame(m: &Vec<u8>, v: u8, a: usize, r: Option<u8>) -> Frame {
    // Count of routine local variables
    let var_count = m[a];
    let mut local_variables = Vec::new();

    // Load local variable values
    for i in 0..var_count {
        if v < 5 {
            local_variables.push(word_value(a + 1 + (i * 2)));
        } else {
            local_variables.push(0 as u16);
        }
    }

    let pc = a + 1 + 2 * var_count;

    Frame {
        pc: pc,
        stack: Vec::new(),
        local_variables: local_variables,
        result_var: r
    }
}