use cpca::AddressParser;
fn main() {
    let p = AddressParser::new();
    let cases = ["云南大理", "四川甘孜", "四川康定", "云南省大理市", "大理市", "甘孜康定"];
    for c in cases {
        let r = p.parse(c);
        println!("{:20} => 省:{:?} 市:{:?} 区:{:?}", c, r.province.as_deref().unwrap_or("-"), r.city.as_deref().unwrap_or("-"), r.district.as_deref().unwrap_or("-"));
    }
}
