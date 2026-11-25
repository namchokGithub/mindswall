use std::path::Path;

fn main() {
    // Test folder path, replace with your own path.
    let folder = Path::new(r"D:\coding\mindswall\example.wallpaper");
    for image in mindswall_core::scanner::scan(folder) {
        println!("{}", image.display());
    }
}
