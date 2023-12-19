use crate::gitr_errors::GitrError;

pub fn get_offset(data: &[u8]) -> Result<(usize, usize), GitrError> {
    let mut ofs: usize = 0;
    let mut cant_bytes: usize = 0;

    for byte in data {
        cant_bytes += 1;
        ofs = (ofs << 7) | (byte & 0x7f) as usize;
        if byte & 0x80 == 0 {
            break;
        }
        ofs += 1
    }
    Ok((ofs, cant_bytes))
}

fn parse_copy_instruction(instruction: Vec<u8>) -> Result<(usize, usize, usize), GitrError> {
    let mut size: usize = 0;
    let mut ofs: usize = 0;
    let activator = instruction[0];
    let tamanio = (activator.count_ones() - 1) as usize;
    let mut i: usize = 0;
    let mut j: usize = 0;
    while i < 7 {
        if i < 3 {
            if (activator & (64 >> i)) != 0 {
                size = (size << 8) | instruction[tamanio - j] as usize;
                j += 1;
            } else {
                size <<= 8;
            }
        } else if i >= 3 {
            if (activator & (64 >> i)) != 0 {
                ofs = (ofs << 8) | instruction[tamanio - j] as usize;
                j += 1;
            } else {
                ofs <<= 8;
            }
        }
        i += 1;
    }
    Ok((ofs, size, j))
}

pub fn transform_delta(data: &[u8], base: &[u8]) -> Result<Vec<u8>, GitrError> {
    let mut final_data: Vec<u8> = Vec::new();
    let mut i: usize = 1;
    for b in base {
        if vec![*b] == ("\0".as_bytes()) {
            break;
        }
        i += 1;
    }
    let base = &base[i..];
    i = 0;
    while i < data.len() {
        let byte = data[i];
        if byte & 0x80 == 0 {
            // empieza con 0 -> nueva data
            let size = (byte << 1 >> 1) as usize;
            let new_data = &data[i + 1..i + 1 + size];
            final_data.extend(new_data);
            i += size + 1;
        } else {
            // empieza con 1 -> copiar de la base
            let (ofs, size, tamanio) = parse_copy_instruction(data[i..].to_vec())?;
            let base_data = &base[ofs..ofs + size];
            final_data.extend(base_data);
            i += 1 + tamanio;
        }
    }
    Ok(final_data)
}
