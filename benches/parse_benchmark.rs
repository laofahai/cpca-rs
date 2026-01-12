use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cpca::AddressParser;

fn benchmark_parse(c: &mut Criterion) {
    let parser = AddressParser::new();

    c.bench_function("parse_full_address", |b| {
        b.iter(|| parser.parse(black_box("广东省深圳市南山区科技园路1号")))
    });

    c.bench_function("parse_short_address", |b| {
        b.iter(|| parser.parse(black_box("深圳南山科技园")))
    });

    c.bench_function("parse_municipality", |b| {
        b.iter(|| parser.parse(black_box("北京市朝阳区望京")))
    });

    c.bench_function("normalize", |b| {
        b.iter(|| parser.normalize(black_box("广东"), black_box("深圳"), black_box(Some("南山"))))
    });
}

fn benchmark_batch(c: &mut Criterion) {
    let parser = AddressParser::new();
    let addresses: Vec<&str> = vec![
        "广东省深圳市南山区",
        "北京市朝阳区",
        "上海市浦东新区",
        "浙江省杭州市西湖区",
        "江苏省南京市鼓楼区",
        "四川省成都市武侯区",
        "湖北省武汉市洪山区",
        "山东省青岛市市南区",
        "福建省厦门市思明区",
        "广东省广州市天河区",
    ];

    c.bench_function("parse_batch_10", |b| {
        b.iter(|| parser.parse_batch(black_box(&addresses)))
    });
}

fn benchmark_init(c: &mut Criterion) {
    c.bench_function("parser_init", |b| {
        b.iter(|| AddressParser::new())
    });
}

criterion_group!(benches, benchmark_parse, benchmark_batch, benchmark_init);
criterion_main!(benches);
