mod path;

use crate::path::{
    PathEnvVar,
    PrintType,
};

fn main() {
    let path = PathEnvVar::new(PrintType::Cleaned);
    path.print();
    path.validate();
}
