use cpca::{AddressParser, RecognitionStatus as Status};

#[test]
fn current_history_and_places_have_separate_statuses() {
    let p = AddressParser::global();
    for (input, district, status, suggestion) in [
        ("中山市南朗街道人民路1号", "南朗街道", Status::Current, None),
        (
            "中山市南朗镇人民路1号",
            "南朗镇",
            Status::Historical,
            Some("南朗街道"),
        ),
        ("中山市民众街道人民路1号", "民众街道", Status::Current, None),
        ("重庆市两江新区人民路1号", "两江新区", Status::Current, None),
        ("重庆市渝北区人民路1号", "渝北区", Status::Historical, None),
        ("宁波市江北区人民路1号", "江北区", Status::Current, None),
        (
            "儋州市国营蓝洋农场人民路1号",
            "国营蓝洋农场",
            Status::Place,
            None,
        ),
        (
            "嘉峪关市镜铁区人民路1号",
            "镜铁区",
            Status::HistoricalManagement,
            None,
        ),
        (
            "嘉峪关市第一街道人民路1号",
            "第一街道",
            Status::NeedsReview,
            None,
        ),
        ("和安县人民路1号", "和安县", Status::Current, None),
        ("和康县人民路1号", "和康县", Status::Current, None),
        ("岑岭县人民路1号", "岑岭县", Status::Current, None),
        (
            "大厂区凤北路22号",
            "大厂区",
            Status::Historical,
            Some("六合区"),
        ),
        (
            "梅列区人民路1号",
            "梅列区",
            Status::Historical,
            Some("三元区"),
        ),
    ] {
        let r = p.parse_with_status(input);
        assert_eq!(r.address.district.as_deref(), Some(district), "{input}");
        assert_eq!(r.validation.status, status, "{input}");
        assert_eq!(r.validation.current_name.as_deref(), suggestion, "{input}");
    }
}

#[test]
fn conflicts_are_not_legitimized_and_partial_results_are_not_current() {
    let p = AddressParser::global();
    for text in [
        "广东省雁塔区人民路1号",
        "上海市渝北区人民路1号",
        "广东省西安市雁塔区人民路1号",
    ] {
        let r = p.parse_with_status(text);
        assert_eq!(r.validation.status, Status::NeedsReview, "{text}");
        assert!(!r.validation.note.is_empty());
    }
    assert_eq!(
        p.parse_with_status("深圳市").validation.status,
        Status::Incomplete
    );
    assert_eq!(
        p.parse_with_status("合作共赢").validation.status,
        Status::Incomplete
    );
    assert_eq!(
        p.parse_with_status("").validation.status,
        Status::Incomplete
    );
}

#[test]
fn ambiguous_districts_return_candidates_without_arbitrary_city() {
    let r = cpca::parse_with_status("江苏省鼓楼区人民路1号");
    assert_eq!(r.address.city, None);
    assert_eq!(r.validation.status, Status::Ambiguous);
    let cities: Vec<_> = r
        .validation
        .candidates
        .iter()
        .map(|r| r.city.as_str())
        .collect();
    assert_eq!(cities, ["南京市", "徐州市"]);
}

#[test]
fn explicit_aliases_target_current_divisions() {
    for (text, city, district) in [
        ("黔南都匀市人民路1号", "黔南布依族苗族自治州", "都匀市"),
        ("呼市新城区人民路1号", "呼和浩特市", "新城区"),
        ("西双版纳景洪市人民路1号", "西双版纳傣族自治州", "景洪市"),
        (
            "兰坪县人民路1号",
            "怒江傈僳族自治州",
            "兰坪白族普米族自治县",
        ),
    ] {
        let r = cpca::parse_with_status(text);
        assert_eq!(r.address.city.as_deref(), Some(city), "{text}");
        assert_eq!(r.address.district.as_deref(), Some(district), "{text}");
        assert_eq!(r.validation.status, Status::Current);
    }
}

#[test]
fn validation_requires_canonical_full_names() {
    let r = cpca::ParsedAddress {
        province: Some("广东省".into()),
        city: Some("深圳市".into()),
        district: Some("南山".into()),
        detail: String::new(),
    };
    assert_eq!(
        AddressParser::global().validate(&r).status,
        Status::NeedsReview
    );
}

#[test]
fn every_bundled_full_address_round_trips() {
    let parser = AddressParser::global();
    for data in [
        include_str!("../data/pca.csv"),
        include_str!("../data/pca_legacy.csv"),
    ] {
        for line in data.lines().skip(1) {
            let fields: Vec<_> = line.split(',').collect();
            if fields[3].is_empty() {
                continue;
            }
            for input in [
                format!("{}{}{}人民路1号", fields[1], fields[2], fields[3]),
                format!("{}{}人民路1号", fields[2], fields[3]),
            ] {
                let r = parser.parse(&input);
                assert_eq!(r.province.as_deref(), Some(fields[1]), "{input}");
                assert_eq!(r.city.as_deref(), Some(fields[2]), "{input}");
                assert_eq!(r.district.as_deref(), Some(fields[3]), "{input}");
                assert_eq!(r.detail, "人民路1号", "{input}");
            }
        }
    }
}

#[test]
fn reviewed_county_aliases_accept_non_road_details() {
    for (text, district, detail) in [
        ("兰坪县人民医院", "兰坪白族普米族自治县", "人民医院"),
        ("元江县第一中学", "元江哈尼族彝族傣族自治县", "第一中学"),
        ("前郭县政府", "前郭尔罗斯蒙古族自治县", "政府"),
    ] {
        let r = cpca::parse_with_status(text);
        assert_eq!(r.address.district.as_deref(), Some(district), "{text}");
        assert_eq!(r.address.detail, detail, "{text}");
        assert_eq!(r.validation.status, Status::Current);
    }
}

#[test]
fn reverse_district_province_city_does_not_hide_explicit_conflict() {
    let r = cpca::parse_with_status("南山区广东省北京市科技园");
    assert_eq!(r.address.province.as_deref(), Some("广东省"));
    assert_eq!(r.address.city.as_deref(), Some("北京市"));
    assert_eq!(r.validation.status, Status::NeedsReview);
}

#[test]
fn all_explicit_aliases_resolve_to_their_reviewed_targets() {
    let p = AddressParser::global();
    for line in include_str!("../data/pca_aliases.csv").lines().skip(1) {
        let f: Vec<_> = line.split(',').collect();
        let input = format!("{}人民路1号", f[0]);
        let r = p.parse(&input);
        assert_eq!(r.province.as_deref(), Some(f[2]), "{input}");
        assert_eq!(r.city.as_deref(), Some(f[3]), "{input}");
        if f[1] == "area" {
            assert_eq!(r.district.as_deref(), Some(f[4]), "{input}");
        }
        assert_eq!(r.detail, "人民路1号", "{input}");
    }
}
