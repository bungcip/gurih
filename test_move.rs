fn main() {
    let s = String::from("hello");
    let v = &s;
    let n = 10;
    if n.to_string() == *v {
        println!("Equal");
    } else {
        println!("Not Equal");
    }
}
