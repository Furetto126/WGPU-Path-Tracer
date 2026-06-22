fn main() {
    let compiler = wesl::Wesl::new("src/shaders/"); 
    // Compute
    compiler.build_artifact(&"package::package.wesl".parse().unwrap(), "path_tracer");
}