use cpca::AddressParser;

fn main() {
    let parser = AddressParser::new();

    println!("=== CPCA 地址解析演示 ===\n");

    let test_cases = vec![
        // 完整地址
        "广东省深圳市南山区科技园路1号",
        "北京市朝阳区望京SOHO",
        "上海市浦东新区陆家嘴金融中心",
        // 简称
        "广东深圳南山科技园",
        "深圳市南山区",
        "深圳南山",
        // 只有市+区（无省）
        "杭州市西湖区",
        "成都武侯区",
        // 只有城市
        "深圳市某某路123号",
        // 直辖市
        "北京朝阳区",
        "上海徐汇区漕河泾",
        "重庆渝中区解放碑",
        "天津市南开区",
        // 自治区
        "广西南宁市青秀区",
        "内蒙古呼和浩特市",
        "新疆乌鲁木齐市天山区",
        // 自治州
        "云南省大理白族自治州大理市",
        "四川省甘孜藏族自治州康定市",
        // 不设区的市
        "广东省东莞市长安镇",
        "广东省中山市小榄镇",
        // 无法识别
        "某某路123号",
        "",
    ];

    for addr in test_cases {
        let result = parser.parse(addr);
        println!("输入: \"{}\"", addr);
        println!("  省份: {:?}", result.province.as_deref().unwrap_or("-"));
        println!("  城市: {:?}", result.city.as_deref().unwrap_or("-"));
        println!("  区县: {:?}", result.district.as_deref().unwrap_or("-"));
        println!("  详址: \"{}\"", result.detail);
        println!("  完整: {}", result.is_complete());
        println!();
    }

    println!("=== 地址标准化演示 ===\n");

    let normalize_cases = vec![
        ("广东", "深圳", Some("南山")),
        ("广东省", "深圳市", Some("南山区")),
        ("北京", "北京", Some("朝阳")),
        ("浙江", "杭州", None),
    ];

    for (p, c, d) in normalize_cases {
        let result = parser.normalize(p, c, d);
        println!("normalize(\"{}\", \"{}\", {:?}) => \"{}\"", p, c, d, result);
    }
}
