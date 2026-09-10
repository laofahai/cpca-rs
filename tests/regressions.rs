use cpca::AddressParser;

#[test]
fn complete_city_name_wins_over_short_district() {
    let r = AddressParser::global().parse("中山市小榄镇人民路1号");
    assert_eq!(r.city.as_deref(), Some("中山市"));
    assert_eq!(r.district.as_deref(), Some("小榄镇"));
    assert_eq!(r.detail, "人民路1号");
}

#[test]
fn explicit_province_is_not_overwritten_by_unique_district() {
    let r = AddressParser::global().parse("广东省雁塔区人民路1号");
    assert_eq!(r.province.as_deref(), Some("广东省"));
    assert_eq!(r.city, None);
    assert_eq!(r.district.as_deref(), Some("雁塔区"));
}

#[test]
fn ordinary_words_are_not_county_aliases() {
    let r = AddressParser::global().parse("合作共赢");
    assert_eq!(r.province, None);
    assert_eq!(r.city, None);
    assert_eq!(r.district, None);
    assert_eq!(r.detail, "合作共赢");
}

#[test]
fn reverse_parent_after_separator_resolves_district() {
    let r = AddressParser::global().parse("朝阳 北京市望京1号");
    assert_eq!(r.province.as_deref(), Some("北京市"));
    assert_eq!(r.city.as_deref(), Some("北京市"));
    assert_eq!(r.district.as_deref(), Some("朝阳区"));
    assert_eq!(r.detail, "望京1号");
}

#[test]
fn hierarchy_and_road_boundaries() {
    for (text, province, city, district, detail) in [
        (
            "黑龙江省西安人民路1号",
            "黑龙江省",
            "牡丹江市",
            "西安区",
            "人民路1号",
        ),
        (
            "江苏省新北人民路1号",
            "江苏省",
            "常州市",
            "新北区",
            "人民路1号",
        ),
        (
            "湖南省资阳人民路1号",
            "湖南省",
            "益阳市",
            "资阳区",
            "人民路1号",
        ),
        (
            "台州路桥区人民路1号",
            "浙江省",
            "台州市",
            "路桥区",
            "人民路1号",
        ),
        (
            "朝阳-北京市望京1号",
            "北京市",
            "北京市",
            "朝阳区",
            "望京1号",
        ),
        ("辽宁省朝阳 北京路1号", "辽宁省", "朝阳市", "", "北京路1号"),
        (
            "西安市陕西雁塔区人民路1号",
            "陕西省",
            "西安市",
            "雁塔区",
            "人民路1号",
        ),
        ("上海市山西北路1号", "上海市", "上海市", "", "山西北路1号"),
        (
            "启东市台湾风情街1号",
            "江苏省",
            "南通市",
            "启东市",
            "台湾风情街1号",
        ),
    ] {
        let r = AddressParser::global().parse(text);
        assert_eq!(r.province.as_deref().unwrap_or(""), province, "{text}");
        assert_eq!(r.city.as_deref().unwrap_or(""), city, "{text}");
        assert_eq!(r.district.as_deref().unwrap_or(""), district, "{text}");
        assert_eq!(r.detail, detail, "{text}");
    }
}

#[test]
fn reverse_parent_aliases_do_not_consume_building_names() {
    for (text, province, city, detail) in [
        ("长春朝阳区吉林大学", "吉林省", "长春市", "吉林大学"),
        ("深圳市南山区山东大厦", "广东省", "深圳市", "山东大厦"),
    ] {
        let r = AddressParser::global().parse(text);
        assert_eq!(r.province.as_deref(), Some(province), "{text}");
        assert_eq!(r.city.as_deref(), Some(city), "{text}");
        assert_eq!(r.detail, detail, "{text}");
    }
    let r = AddressParser::global().parse("南山区深圳大学");
    assert_eq!(r.city, None);
    assert_eq!(r.detail, "深圳大学");
}

#[test]
fn explicit_county_suffix_wins_over_city_alias_with_province() {
    let r = AddressParser::global().parse("辽宁省朝阳县人民路1号");
    assert_eq!(r.city.as_deref(), Some("朝阳市"));
    assert_eq!(r.district.as_deref(), Some("朝阳县"));
    assert_eq!(r.detail, "人民路1号");
}

#[test]
fn province_alias_before_full_district_is_not_a_road() {
    for (text, province, city, district) in [
        ("浙江路桥区人民路1号", "浙江省", "台州市", "路桥区"),
        ("河北路南区人民路1号", "河北省", "唐山市", "路南区"),
    ] {
        let r = AddressParser::global().parse(text);
        assert_eq!(r.province.as_deref(), Some(province), "{text}");
        assert_eq!(r.city.as_deref(), Some(city), "{text}");
        assert_eq!(r.district.as_deref(), Some(district), "{text}");
        assert_eq!(r.detail, "人民路1号");
    }
    let r = AddressParser::global().parse("浙江路1号");
    assert_eq!(r.province, None);
    assert_eq!(r.detail, "浙江路1号");
}
