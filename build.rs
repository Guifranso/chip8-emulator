fn main() {
    // Avisa o Rust para procurar bibliotecas (.lib) na pasta atual do projeto
    println!("cargo:rustc-link-search=native=.");
}