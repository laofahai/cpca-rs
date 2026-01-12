use cpca::AddressParser;
fn main() {
    let p = AddressParser::new();
    // 县级市测试
    let cases = [
        "康定市",    // 甘孜州下的县级市
        "大理市",    // 大理州下的县级市
        "义乌市",    // 金华市下的县级市
        "昆山市",    // 苏州市下的县级市
        "寿光市",    // 潍坊市下的县级市
        "晋江市",    // 泉州市下的县级市
        "南山区",    // 深圳市下的区
        "朝阳区",    // 多个城市都有（北京、长春）
    ];
    for c in cases {
        let r = p.parse(c);
        println!("{:12} => 省:{:?} 市:{:?} 区:{:?}", 
            c, 
            r.province.as_deref().unwrap_or("-"), 
            r.city.as_deref().unwrap_or("-"), 
            r.district.as_deref().unwrap_or("-"));
    }
}
