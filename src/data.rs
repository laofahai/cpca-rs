//! 省市区数据加载和索引构建

use crate::region::Region;
use std::collections::{HashMap, HashSet};

/// 内嵌的省市区数据（编译时包含）
const PCA_DATA: &str = include_str!("../data/pca.csv");

/// 直辖市列表
pub const MUNICIPALITIES: [&str; 4] = ["北京市", "上海市", "天津市", "重庆市"];

/// 不设区的地级市（直筒子市）
#[allow(dead_code)]
pub const NO_DISTRICT_CITIES: [&str; 4] = ["东莞市", "中山市", "儋州市", "嘉峪关市"];

/// 省份简称映射
pub fn province_aliases() -> HashMap<&'static str, &'static str> {
    let mut map = HashMap::new();
    // 省份
    map.insert("广东", "广东省");
    map.insert("江苏", "江苏省");
    map.insert("浙江", "浙江省");
    map.insert("山东", "山东省");
    map.insert("河南", "河南省");
    map.insert("河北", "河北省");
    map.insert("四川", "四川省");
    map.insert("湖北", "湖北省");
    map.insert("湖南", "湖南省");
    map.insert("福建", "福建省");
    map.insert("安徽", "安徽省");
    map.insert("江西", "江西省");
    map.insert("陕西", "陕西省");
    map.insert("山西", "山西省");
    map.insert("辽宁", "辽宁省");
    map.insert("吉林", "吉林省");
    map.insert("黑龙江", "黑龙江省");
    map.insert("云南", "云南省");
    map.insert("贵州", "贵州省");
    map.insert("甘肃", "甘肃省");
    map.insert("海南", "海南省");
    map.insert("青海", "青海省");
    map.insert("台湾", "台湾省");

    // 自治区
    map.insert("广西", "广西壮族自治区");
    map.insert("内蒙古", "内蒙古自治区");
    map.insert("西藏", "西藏自治区");
    map.insert("新疆", "新疆维吾尔自治区");
    map.insert("宁夏", "宁夏回族自治区");

    // 直辖市
    map.insert("北京", "北京市");
    map.insert("上海", "上海市");
    map.insert("天津", "天津市");
    map.insert("重庆", "重庆市");

    // 特别行政区
    map.insert("香港", "香港特别行政区");
    map.insert("澳门", "澳门特别行政区");

    map
}

/// 城市简称（去掉"市"后缀）
#[allow(dead_code)]
pub fn normalize_city_name(city: &str) -> String {
    if city.ends_with("市")
        || city.ends_with("自治州")
        || city.ends_with("地区")
        || city.ends_with("盟")
    {
        city.to_string()
    } else {
        format!("{}市", city)
    }
}

/// 区县简称处理
#[allow(dead_code)]
pub fn normalize_district_name(district: &str) -> String {
    if district.ends_with("区")
        || district.ends_with("县")
        || district.ends_with("市")
        || district.ends_with("旗")
    {
        district.to_string()
    } else {
        // 尝试添加"区"
        format!("{}区", district)
    }
}

/// 加载并解析 CSV 数据
pub fn load_regions() -> Vec<Region> {
    let mut regions = Vec::new();

    for line in PCA_DATA.lines().skip(1) {
        // 跳过表头
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 3 {
            let province = parts[1].trim().to_string();
            let city = parts[2].trim().to_string();
            let district = if parts.len() > 3 && !parts[3].trim().is_empty() {
                Some(parts[3].trim().to_string())
            } else {
                None
            };

            if !province.is_empty() && !city.is_empty() {
                regions.push(Region::new(province, city, district));
            }
        }
    }

    regions
}

/// 构建各种索引
pub struct RegionIndex {
    /// 所有省份集合
    pub provinces: HashSet<String>,
    /// 省份 -> 城市列表
    pub province_cities: HashMap<String, HashSet<String>>,
    /// 城市 -> 省份（反向索引）
    pub city_to_province: HashMap<String, String>,
    /// 城市 -> 区县列表
    pub city_districts: HashMap<String, HashSet<String>>,
    /// 区县 -> (省份, 城市) 反向索引
    pub district_to_city: HashMap<String, Vec<(String, String)>>,
    /// 所有城市集合
    pub cities: HashSet<String>,
    /// 所有区县集合
    pub districts: HashSet<String>,
}

impl RegionIndex {
    /// 从地区列表构建索引
    pub fn build(regions: &[Region]) -> Self {
        let mut provinces = HashSet::new();
        let mut province_cities: HashMap<String, HashSet<String>> = HashMap::new();
        let mut city_to_province = HashMap::new();
        let mut city_districts: HashMap<String, HashSet<String>> = HashMap::new();
        let mut district_to_city: HashMap<String, Vec<(String, String)>> = HashMap::new();
        let mut cities = HashSet::new();
        let mut districts = HashSet::new();

        for region in regions {
            // 省份
            provinces.insert(region.province.clone());

            // 省份 -> 城市
            province_cities
                .entry(region.province.clone())
                .or_default()
                .insert(region.city.clone());

            // 城市 -> 省份
            city_to_province.insert(region.city.clone(), region.province.clone());
            cities.insert(region.city.clone());

            // 城市简称也加入索引
            if region.city.ends_with("市") {
                let short = region.city.trim_end_matches("市");
                city_to_province.insert(short.to_string(), region.province.clone());
            }

            // 区县
            if let Some(ref district) = region.district {
                city_districts
                    .entry(region.city.clone())
                    .or_default()
                    .insert(district.clone());

                district_to_city
                    .entry(district.clone())
                    .or_default()
                    .push((region.province.clone(), region.city.clone()));

                districts.insert(district.clone());

                // 区县简称也加入索引
                for suffix in &["区", "县", "市", "旗"] {
                    if district.ends_with(suffix) {
                        let short = district.trim_end_matches(suffix);
                        if !short.is_empty() {
                            district_to_city
                                .entry(short.to_string())
                                .or_default()
                                .push((region.province.clone(), region.city.clone()));
                        }
                    }
                }
            }
        }

        Self {
            provinces,
            province_cities,
            city_to_province,
            city_districts,
            district_to_city,
            cities,
            districts,
        }
    }

    /// 检查是否是直辖市
    pub fn is_municipality(&self, province: &str) -> bool {
        MUNICIPALITIES.contains(&province)
    }

    /// 检查是否是不设区的市
    #[allow(dead_code)]
    pub fn is_no_district_city(&self, city: &str) -> bool {
        NO_DISTRICT_CITIES.contains(&city)
    }

    /// 根据城市查找省份
    #[allow(dead_code)]
    pub fn find_province_by_city(&self, city: &str) -> Option<&String> {
        self.city_to_province.get(city)
    }

    /// 根据区县查找可能的城市（可能有多个同名区县）
    #[allow(dead_code)]
    pub fn find_cities_by_district(&self, district: &str) -> Option<&Vec<(String, String)>> {
        self.district_to_city.get(district)
    }

    /// 验证区县是否属于某个城市
    pub fn validate_district(&self, city: &str, district: &str) -> bool {
        self.city_districts
            .get(city)
            .map(|ds| ds.contains(district))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_regions() {
        let regions = load_regions();
        assert!(!regions.is_empty());
        // 应该有3000+条记录
        assert!(regions.len() > 3000);
    }

    #[test]
    fn test_province_aliases() {
        let aliases = province_aliases();
        assert_eq!(aliases.get("广东"), Some(&"广东省"));
        assert_eq!(aliases.get("北京"), Some(&"北京市"));
        assert_eq!(aliases.get("内蒙古"), Some(&"内蒙古自治区"));
    }

    #[test]
    fn test_region_index() {
        let regions = load_regions();
        let index = RegionIndex::build(&regions);

        // 检查省份
        assert!(index.provinces.contains("广东省"));
        assert!(index.provinces.contains("北京市"));

        // 检查城市索引
        assert!(index.cities.contains("深圳市"));
        assert_eq!(
            index.city_to_province.get("深圳市"),
            Some(&"广东省".to_string())
        );

        // 检查直辖市
        assert!(index.is_municipality("北京市"));
        assert!(!index.is_municipality("广东省"));

        // 检查不设区的市
        assert!(index.is_no_district_city("东莞市"));
        assert!(!index.is_no_district_city("深圳市"));
    }
}
