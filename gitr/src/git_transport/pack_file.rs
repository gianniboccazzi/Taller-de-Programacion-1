extern crate flate2;

use std::collections::HashMap;
use std::io::Write;
use std::fmt::Write as FmtWrite;

use crate::commands::command_utils::*;
use crate::git_transport::{deltas::*, ref_discovery::*};
use crate::gitr_errors::{self, GitrError};
use crate::objects::blob::Blob;
use crate::objects::commit::Commit;
use crate::objects::git_object::GitObject;
use crate::objects::tag::Tag;
use crate::objects::tree::Tree;
use flate2::write::ZlibEncoder;
use flate2::{Compression, Decompress};
#[derive(Debug)]
pub struct PackFile {
    _version: u32,
    pub objects: Vec<GitObject>,
}

pub fn decode(input: &[u8]) -> Result<(Vec<u8>, u64), GitrError> {
    let mut decoder = Decompress::new(true);
    let mut output: [u8; 1024] = [0; 1024];
    if decoder
        .decompress(input, &mut output, flate2::FlushDecompress::Finish)
        .is_err()
    {
        return Err(GitrError::CompressionError);
    }
    let cant_leidos = decoder.total_in();
    let output_return = output[..decoder.total_out() as usize].to_vec();

    Ok((output_return, cant_leidos))
}

pub fn code(input: &[u8]) -> Result<Vec<u8>, GitrError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    if encoder.write_all(input).is_err() {
        return Err(GitrError::CompressionError);
    }
    match encoder.finish() {
        Ok(compressed) => Ok(compressed),
        Err(_e) => Err(GitrError::CompressionError),
    }
}

fn parse_git_object(data: &[u8]) -> Result<(u8, usize, &[u8], usize), GitrError> {
    if data.len() < 2 {
        return Err(GitrError::PackFileError(
            "parse_git_object".to_string(),
            "No hay suficientes bytes para el encabezado mínimo".to_string(),
        ));
    }
    let object_type = (data[0] << 1 >> 5) & 0x07;
    let (length, cursor) = get_encoded_length(data)?;
    let object_content = &data[cursor..];
    Ok((object_type, length, object_content, cursor))
}

fn get_encoded_length(data: &[u8]) -> Result<(usize, usize), GitrError> {
    let mut length = (data[0] << 4 >> 4) as usize;
    let mut cursor = 1;
    let mut shift = 4;
    while (data[cursor - 1] & 0x80) != 0 {
        length |= (data[cursor] as usize & 0x7F) << shift;
        shift += 7;
        if shift > 28 {
            return Err(GitrError::PackFileError(
                "parse_git_object".to_string(),
                "La longitud es demasiado grande".to_string(),
            ));
        }
        cursor += 1;
    }
    Ok((length, cursor))
}

fn create_commit_object(decoded_data: &[u8]) -> Result<GitObject, GitrError> {
    let commit = Commit::new_commit_from_string(String::from_utf8_lossy(decoded_data).to_string())?;
    Ok(GitObject::Commit(commit))
}

fn create_tree_object(decoded_data: &[u8]) -> Result<GitObject, GitrError> {
    let tree = GitObject::Tree(Tree::new_from_packfile(decoded_data)?);
    Ok(tree)
}

fn create_tag_object(decoded_data: &[u8]) -> Result<GitObject, GitrError> {
    let tag = GitObject::Tag(Tag::new_tag_from_string(
        String::from_utf8_lossy(decoded_data).to_string(),
    )?);
    Ok(tag)
}

fn create_blob_object(decoded_data: &[u8]) -> Result<GitObject, GitrError> {
    let data_str = String::from_utf8_lossy(decoded_data);
    let blob = GitObject::Blob(Blob::new(data_str.to_string())?);

    Ok(blob)
}

fn git_valid_object_from_packfile(
    object_type: u8,
    decoded_data: &[u8],
) -> Result<GitObject, GitrError> {
    let object = match object_type {
        1 => create_commit_object(decoded_data)?,
        2 => create_tree_object(decoded_data)?,
        3 => create_blob_object(decoded_data)?,
        4 => create_tag_object(decoded_data)?,
        _ => {
            return Err(GitrError::PackFileError(
                "git_valid_object_from_packfile".to_string(),
                "Tipo de objeto no válido".to_string(),
            ))
        }
    };
    Ok(object)
}

pub fn read_pack_file(buffer: &mut [u8]) -> Result<Vec<GitObject>, GitrError> {
    let num_objects = match buffer[8..12].try_into() {
        Ok(vec) => vec,
        Err(_e) => {
            return Err(gitr_errors::GitrError::PackFileError(
                "read_pack_file".to_string(),
                "no se pudo obtener la # objetos".to_string(),
            ))
        }
    };
    let num_objects = u32::from_be_bytes(num_objects);
    let mut objects = vec![];

    let mut index: usize = 12;
    let mut hash_objects: HashMap<String, (u8, Vec<u8>)> = HashMap::new();
    for _i in 0..num_objects {
        let (obj, leidos) = read_object(buffer, index, &mut hash_objects)?;
        index += leidos;
        objects.push(obj);
    }
    Ok(objects)
}

pub fn read_object(
    buffer: &[u8],
    index: usize,
    objects_dir: &mut HashMap<String, (u8, Vec<u8>)>,
) -> Result<(GitObject, usize), GitrError> {
    match parse_git_object(&buffer[index..]) {
        Ok((object_type, _length, object_content, cursor)) => {
            let (obj, leidos): (GitObject, usize);
            if object_type == 6 {
                (obj, leidos) =
                    delta_ofs_from_packfile(object_content, buffer, index, objects_dir)?;
            } else if object_type == 7 {
                (obj, leidos) = delta_ref_from_packfile(object_content, objects_dir)?;
            } else {
                let (decodeado, l) = decode(object_content)?;
                leidos = l as usize;
                obj = git_valid_object_from_packfile(object_type, &decodeado)?;
            }
            objects_dir.insert(obj.get_hash().to_string(), (obj.get_type(), obj.get_data()));
            Ok((obj, leidos + cursor))
        }
        Err(err) => {
            println!("Error: {}", err);
            Err(GitrError::PackFileError(
                "read_pack_file".to_string(),
                "no se pudo parsear el objeto".to_string(),
            ))
        }
    }
}

fn delta_ofs_from_packfile(
    object_content: &[u8],
    buffer: &[u8],
    index: usize,
    objects_dir: &mut HashMap<String, (u8, Vec<u8>)>,
) -> Result<(GitObject, usize), GitrError> {
    let (ofs, c1) = get_offset(object_content)?; // primero esta el offset
    let (delta_decoded, c2) = decode(&object_content[c1..])?; // descomprimo el delta
    let (_length, c3) = get_encoded_length(&delta_decoded)?; // despues la longitud del obj base
    let (_length, c4) = get_encoded_length(&delta_decoded[c3..])?; // despues la longitud del obj resultante
    let base_git_object = read_object(buffer, index - ofs, objects_dir)?.0; // busco el objeto base
    let base = decode(&base_git_object.get_data())?.0; // le saco la data al objeto base
    let base_type = base_git_object.get_type(); // obtengo el tipo del objeto base
    let reconstructed = transform_delta(&delta_decoded[c3 + c4..], &base)?; // la reconstruyo
    let obj = git_valid_object_from_packfile(base_type, &reconstructed)?; // la parseo
    Ok((obj, c1 + c2 as usize))
}

fn delta_ref_from_packfile(
    object_content: &[u8],
    objects_dir: &mut HashMap<String, (u8, Vec<u8>)>,
) -> Result<(GitObject, usize), GitrError> {
    let hex_string: String = object_content[..20]
        .iter()
        .fold(String::new(),|mut output,b| {
            let _ =write!(output,"{b:02x}");
            output
        });
    let (delta_decoded, c1) = decode(&object_content[20..])?; // descomprimo el delta
    let (_length, c2) = get_encoded_length(&delta_decoded)?; // despues la longitud del obj base
    let (_length, c3) = get_encoded_length(&delta_decoded[c2..])?; // despues la longitud del obj resultante
    if let Some(b) = objects_dir.get(&hex_string) {
        let base = decode(&b.1)?.0;
        let base_type = b.0;
        let reconstructed = transform_delta(&delta_decoded[c2 + c3..], &base)?; // la reconstruyo
        let obj = git_valid_object_from_packfile(base_type, &reconstructed)?; // la parseo
        return Ok((obj, 20 + c1 as usize));
    }
    Err(GitrError::PackFileError(
        "delta_ref_from_packfile".to_string(),
        "No se encontro el objeto base".to_string(),
    ))
}

pub fn prepare_contents(datos: Vec<Vec<u8>>) -> Vec<(String, String, Vec<u8>)> {
    let mut contents: Vec<(String, String, Vec<u8>)> = Vec::new();
    for data in datos {
        let mut i: usize = 0;
        for byte in data.clone() {
            if byte == b'\0' {
                break;
            }
            i += 1;
        }
        let (header, raw_data) = data.split_at(i);
        let h_str = String::from_utf8_lossy(header).to_string();
        let (obj_type, obj_len) = h_str.split_once(' ').unwrap_or(("", ""));
        let (_, raw_data) = raw_data.split_at(1);
        contents.push((obj_type.to_string(), obj_len.to_string(), raw_data.to_vec()));
    }
    contents
}

/// Recibe vector de strings con los objetos a comprimir y devuelve un vector de bytes con el packfile
pub fn create_packfile(contents: Vec<(String, String, Vec<u8>)>) -> Result<Vec<u8>, GitrError> {
    // ########## HEADER ##########
    let mut final_data: Vec<u8> = Vec::new();
    let header = "PACK".to_string();
    final_data.extend(header.as_bytes());
    let cant_bytes = contents.len().to_be_bytes();
    let ver: u32 = 2;
    final_data.extend(&ver.to_be_bytes());
    final_data.extend(&cant_bytes[4..8]);
    // ########## OBJECTS ##########
    for (obj_type, len, raw_data) in contents {
        let mut obj_data: Vec<u8> = Vec::new();
        let obj_type: u8 = match obj_type.as_str() {
            // obtengo el tipo de objeto
            "commit" => 1,
            "tree" => 2,
            "blob" => 3,
            "tag" => 4,
            _ => {
                return Err(GitrError::PackFileError(
                    "create_packfile".to_string(),
                    "Tipo de objeto no válido".to_string(),
                ))
            }
        };

        let obj_len = match len.parse::<usize>() {
            // obtengo la longitud del objeto
            Ok(len) => len,
            Err(_e) => {
                return Err(GitrError::PackFileError(
                    "create_packfile".to_string(),
                    "Longitud de objeto no válida".to_string(),
                ))
            }
        };
        if obj_len < 16 {
            obj_data.push((obj_type << 4) | obj_len as u8);
        } else {
            // ###### SIZE ENCODING ######
            let mut size = obj_len;
            let mut size_bytes: Vec<u8> = Vec::new();
            size_bytes.push((obj_type << 4) | (size & 0x0F) as u8 | 0x80); // meto el tipo de objeto y los primeros 4 bits de la longitud
            size >>= 4;
            while size >= 128 {
                size_bytes.push((size & 0x7F) as u8 | 0x80); // meto los siguientes 7 bits de la longitud con un 1 adelante
                size >>= 7;
            }
            size_bytes.push(size as u8); // meto los últimos ultimos 7 bits de la longitud con un 0 adelante
            obj_data.extend(size_bytes);
        }
        let compressed = code(&raw_data)?;
        obj_data.extend(compressed);
        final_data.extend(obj_data);
    }

    // ########## CHECKSUM ##########
    let hasheado = sha1hashing2(final_data.clone());
    final_data.extend(&hasheado);

    Ok(final_data)
}

impl PackFile {
    pub fn new_from_server_packfile(buffer: &mut [u8]) -> Result<PackFile, GitrError> {
        if buffer.len() < 32 {
            println!("Error: No hay suficientes bytes para el packfile mínimo, se recibieron {} bytes\n {:?}",buffer.len(),String::from_utf8_lossy(buffer));
            return Err(GitrError::PackFileError(
                "new_from_server_packfile".to_string(),
                "No hay suficientes bytes para el encabezado mínimo".to_string(),
            ));
        }
        verify_header(&buffer[..=3])?;
        let version = extract_version(&buffer[4..=7])?;
        let objects = read_pack_file(buffer)?;

        Ok(PackFile {
            _version: version,
            objects,
        })
    }
}
