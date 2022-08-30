fn main() {
    //TODO
    let g = u64::MAX;
    let (g, _) = g.overflowing_add(1);
    println!("g = {}", g);
}