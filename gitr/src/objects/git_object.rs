use super::blob::Blob;
use super::commit::Commit;
use super::tag::Tag;
use super::tree::Tree;

#[derive(Debug)]

pub enum GitObject {
    Blob(Blob),
    Commit(Commit),
    Tree(Tree),
    Tag(Tag),
}
impl GitObject {
    pub fn get_data(&self) -> Vec<u8> {
        match self {
            GitObject::Blob(blob) => blob.get_data(),
            GitObject::Commit(commit) => commit.get_data(),
            GitObject::Tree(tree) => tree.get_data(),
            GitObject::Tag(tag) => tag.get_data(),
        }
    }
    pub fn get_hash(&self) -> String {
        match self {
            GitObject::Blob(blob) => blob.get_hash(),
            GitObject::Commit(commit) => commit.get_hash(),
            GitObject::Tree(tree) => tree.get_hash(),
            GitObject::Tag(tag) => tag.get_hash(),
        }
    }
    pub fn get_type(&self) -> u8 {
        match self {
            GitObject::Commit(_commit) => 1,
            GitObject::Tree(_tree) => 2,
            GitObject::Blob(_blob) => 3,
            GitObject::Tag(_tag) => 4,
        }
    }
}
