//! 意图分类器模块
//!
//! 提供查询意图识别和实体提取能力，支持中文自然语言理解。
//! 用于 Agentic Search 的前置分析步骤。

use regex::Regex;
use serde::{Deserialize, Serialize};

/// 查询意图类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryIntent {
    /// 学生相关查询
    Student,
    /// 班级整体查询
    Class,
    /// 课程/课堂相关
    Lesson,
    /// 作业/考评相关
    Assignment,
    /// 通用对话（无需检索）
    General,
}

impl QueryIntent {
    /// 获取意图的中文描述
    pub fn description(&self) -> &'static str {
        match self {
            QueryIntent::Student => "学生个人",
            QueryIntent::Class => "班级整体",
            QueryIntent::Lesson => "课程课堂",
            QueryIntent::Assignment => "作业考评",
            QueryIntent::General => "通用对话",
        }
    }
}

/// 提取的实体信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractedEntities {
    /// 学生姓名列表
    pub student_names: Vec<String>,
    /// 班级名称
    pub class_name: Option<String>,
    /// 学科
    pub subject: Option<String>,
    /// 日期范围起始
    pub from_date: Option<String>,
    /// 日期范围截止
    pub to_date: Option<String>,
    /// 关键词
    pub keywords: Vec<String>,
}

/// 意图分类结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentClassification {
    /// 识别出的意图
    pub intent: QueryIntent,
    /// 置信度 (0.0 ~ 1.0)
    pub confidence: f32,
    /// 提取的实体
    pub entities: ExtractedEntities,
    /// 是否需要检索证据
    pub needs_evidence: bool,
}

/// 意图分类器
pub struct IntentClassifier;

impl IntentClassifier {
    /// 创建新的分类器
    pub fn new() -> Self {
        Self
    }

    /// 分类查询意图并提取实体
    ///
    /// 使用规则匹配识别查询类型，支持中文语义理解。
    pub fn classify(&self, query: &str) -> IntentClassification {
        let query = query.trim();
        if query.is_empty() {
            return IntentClassification {
                intent: QueryIntent::General,
                confidence: 1.0,
                entities: ExtractedEntities::default(),
                needs_evidence: false,
            };
        }

        // 提取实体
        let entities = self.extract_entities(query);

        // 基于关键词和实体判断意图
        let (intent, confidence) = self.detect_intent(query, &entities);

        // 判断是否需要检索证据
        let needs_evidence = matches!(
            intent,
            QueryIntent::Student
                | QueryIntent::Class
                | QueryIntent::Lesson
                | QueryIntent::Assignment
        );

        IntentClassification {
            intent,
            confidence,
            entities,
            needs_evidence,
        }
    }

    /// 检测意图类型
    fn detect_intent(&self, query: &str, entities: &ExtractedEntities) -> (QueryIntent, f32) {
        let query_lower = query.to_lowercase();

        // 学生相关关键词
        let student_keywords = [
            "学生",
            "同学",
            "小明",
            "小红",
            "小张",
            "小李",
            "小王",
            "表现",
            "成绩",
            "学习",
            "作业",
            "课堂",
            "纪律",
            "行为",
            "最近",
            "最近怎么样",
            "情况",
            "状态",
            "进步",
            "退步",
        ];

        // 班级相关关键词
        let class_keywords = [
            "班级",
            "全班",
            "整体",
            "平均水平",
            "整体表现",
            "班风",
            "学风",
            "纪律",
            "平均分",
            "排名",
        ];

        // 课程/课堂相关
        let lesson_keywords = [
            "课",
            "课堂",
            "上课",
            "教学",
            "这节课",
            "今天这节课",
            "效果",
            "反馈",
            "互动",
            "参与度",
            "听懂",
            "理解",
        ];

        // 作业/考评相关
        let assignment_keywords = [
            "作业",
            "练习",
            "考试",
            "测验",
            "测评",
            "批改",
            "完成",
            "提交",
            "未交",
            "缺交",
            "质量",
            "正确率",
            "错题",
            "薄弱点",
            "掌握",
            "没掌握",
        ];

        // 计算各意图匹配分数
        let student_score = self.calculate_match_score(&query_lower, &student_keywords);
        let class_score = self.calculate_match_score(&query_lower, &class_keywords);
        let lesson_score = self.calculate_match_score(&query_lower, &lesson_keywords);
        let assignment_score = self.calculate_match_score(&query_lower, &assignment_keywords);

        // 如果有具体学生姓名，优先认为是学生查询
        if !entities.student_names.is_empty() {
            return (QueryIntent::Student, 0.9);
        }

        // 根据最高分数确定意图
        let scores = [
            (QueryIntent::Student, student_score),
            (QueryIntent::Class, class_score),
            (QueryIntent::Lesson, lesson_score),
            (QueryIntent::Assignment, assignment_score),
        ];

        let max_score = scores.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        match max_score {
            Some((intent, score)) if *score > 0.0 => (intent.clone(), *score),
            _ => (QueryIntent::General, 1.0),
        }
    }

    /// 计算匹配分数
    fn calculate_match_score(&self, query: &str, keywords: &[&str]) -> f32 {
        let mut score = 0.0_f32;
        for keyword in keywords {
            if query.contains(keyword) {
                // 更长的关键词匹配权重更高
                let weight = keyword.len() as f32 / 10.0;
                score += 0.1 + weight.min(0.5);
            }
        }
        score.min(1.0)
    }

    /// 提取实体
    fn extract_entities(&self, query: &str) -> ExtractedEntities {
        ExtractedEntities {
            student_names: self.extract_student_names(query),
            class_name: self.extract_class_name(query),
            subject: self.extract_subject(query),
            from_date: self.extract_date_range(query).0,
            to_date: self.extract_date_range(query).1,
            keywords: self.extract_keywords(query),
        }
    }

    /// 提取学生姓名
    ///
    /// 识别常见的中文姓名模式和上下文
    fn extract_student_names(&self, query: &str) -> Vec<String> {
        let mut names = Vec::new();

        // 常见中文姓氏
        let surnames = [
            "张", "王", "李", "刘", "陈", "杨", "黄", "赵", "吴", "周", "徐", "孙", "马", "朱",
            "胡", "郭", "何", "林", "罗", "高", "郑", "梁", "谢", "宋", "唐", "许", "韩", "冯",
            "邓", "曹", "彭", "曾", "肖", "田", "董", "袁", "潘", "于", "蒋", "蔡", "余", "杜",
            "叶", "程", "苏", "魏", "吕", "丁", "任", "沈", "小", "明", "红", "华", "强", "伟",
            "磊", "静", "敏", "丽",
        ];

        // 模式："XX同学"、"XX最近"、"XX的表现"
        let patterns = [
            Regex::new(r"([\u4e00-\u9fa5]{2,4})同学").expect("有效正则表达式：匹配中文姓名+同学"),
            Regex::new(r"([\u4e00-\u9fa5]{2,4})最近").expect("有效正则表达式：匹配中文姓名+最近"),
            Regex::new(r"([\u4e00-\u9fa5]{2,4})的").expect("有效正则表达式：匹配中文姓名+的"),
            Regex::new(r"([\u4e00-\u9fa5]{2,4})(表现|成绩|作业|情况)")
                .expect("有效正则表达式：匹配中文姓名+表现/成绩/作业/情况"),
        ];

        for pattern in &patterns {
            for cap in pattern.captures_iter(query) {
                if let Some(name_match) = cap.get(1) {
                    let name = name_match.as_str();
                    // 检查是否以姓氏开头且不重复
                    if surnames.iter().any(|s| name.starts_with(s))
                        && !names.contains(&name.to_string())
                    {
                        names.push(name.to_string());
                    }
                }
            }
        }

        names
    }

    /// 提取班级名称
    fn extract_class_name(&self, query: &str) -> Option<String> {
        // 模式：X年级X班、X班
        let patterns = [
            Regex::new(r"([一二三四五六七八九十\d]+年级[一二三四五六七八九十\d]+班)").unwrap(),
            Regex::new(r"([一二三四五六七八九十\d]+班)").unwrap(),
        ];

        for pattern in &patterns {
            if let Some(cap) = pattern.captures(query) {
                if let Some(class_match) = cap.get(1) {
                    return Some(class_match.as_str().to_string());
                }
            }
        }

        None
    }

    /// 提取学科
    fn extract_subject(&self, query: &str) -> Option<String> {
        let subjects = [
            ("语文", "语文"),
            ("数学", "数学"),
            ("英语", "英语"),
            ("物理", "物理"),
            ("化学", "化学"),
            ("生物", "生物"),
            ("历史", "历史"),
            ("地理", "地理"),
            ("政治", "政治"),
            ("思想品德", "政治"),
            ("体育", "体育"),
            ("音乐", "音乐"),
            ("美术", "美术"),
            ("科学", "科学"),
            ("信息技术", "信息技术"),
            ("编程", "信息技术"),
            ("计算机", "信息技术"),
        ];

        for (keyword, subject) in &subjects {
            if query.contains(keyword) {
                return Some(subject.to_string());
            }
        }

        None
    }

    /// 提取日期范围
    fn extract_date_range(&self, query: &str) -> (Option<String>, Option<String>) {
        let mut from_date = None;
        let mut to_date = None;

        // 相对时间模式
        if query.contains("最近一周") || query.contains("这周") || query.contains("本周") {
            let now = chrono::Local::now();
            let week_ago = now - chrono::Duration::days(7);
            from_date = Some(week_ago.format("%Y-%m-%d").to_string());
            to_date = Some(now.format("%Y-%m-%d").to_string());
        } else if query.contains("最近一个月") || query.contains("这个月") || query.contains("本月")
        {
            let now = chrono::Local::now();
            let month_ago = now - chrono::Duration::days(30);
            from_date = Some(month_ago.format("%Y-%m-%d").to_string());
            to_date = Some(now.format("%Y-%m-%d").to_string());
        } else if query.contains("最近") || query.contains("近来") {
            // 默认最近两周
            let now = chrono::Local::now();
            let two_weeks_ago = now - chrono::Duration::days(14);
            from_date = Some(two_weeks_ago.format("%Y-%m-%d").to_string());
            to_date = Some(now.format("%Y-%m-%d").to_string());
        }

        // 绝对日期模式 YYYY-MM-DD
        let date_pattern = Regex::new(r"(\d{4}-\d{2}-\d{2})").unwrap();
        let dates: Vec<String> = date_pattern
            .captures_iter(query)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect();

        if dates.len() >= 2 {
            from_date = Some(dates[0].clone());
            to_date = Some(dates[1].clone());
        } else if dates.len() == 1 {
            // 只有一个日期，作为起始日期
            from_date = Some(dates[0].clone());
        }

        (from_date, to_date)
    }

    /// 提取关键词
    fn extract_keywords(&self, query: &str) -> Vec<String> {
        // 停用词列表
        let stop_words = [
            "的",
            "了",
            "在",
            "是",
            "我",
            "有",
            "和",
            "就",
            "不",
            "人",
            "都",
            "一",
            "一个",
            "上",
            "也",
            "很",
            "到",
            "说",
            "要",
            "去",
            "你",
            "会",
            "着",
            "没有",
            "看",
            "好",
            "自己",
            "这",
            "那",
            "最近",
            "怎么样",
            "如何",
            "什么",
            "吗",
            "呢",
            "吧",
            "啊",
        ];

        // 简单分词：按2-4字滑动窗口提取
        let mut words = Vec::new();
        let chars: Vec<char> = query.chars().collect();

        for window_size in 2..=4 {
            for i in 0..chars.len().saturating_sub(window_size - 1) {
                let word: String = chars[i..i + window_size].iter().collect();
                if word.len() >= 4 && !stop_words.contains(&word.as_str()) {
                    words.push(word);
                }
            }
        }

        // 去重并限制数量
        let mut unique = Vec::new();
        for word in words {
            if !unique.contains(&word) && unique.len() < 5 {
                unique.push(word);
            }
        }

        unique
    }
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_student_intent() {
        let classifier = IntentClassifier::new();

        let result = classifier.classify("小明最近怎么样");
        assert_eq!(result.intent, QueryIntent::Student);
        assert!(!result.entities.student_names.is_empty());
        assert!(result.entities.student_names.contains(&"小明".to_string()));
        assert!(result.needs_evidence);
    }

    #[test]
    fn test_classify_class_intent() {
        let classifier = IntentClassifier::new();

        let result = classifier.classify("班级整体表现如何");
        assert_eq!(result.intent, QueryIntent::Class);
        assert!(result.needs_evidence);
    }

    #[test]
    fn test_classify_assignment_intent() {
        let classifier = IntentClassifier::new();

        let result = classifier.classify("作业完成情况");
        assert_eq!(result.intent, QueryIntent::Assignment);
        assert!(result.needs_evidence);
    }

    #[test]
    fn test_extract_subject() {
        let classifier = IntentClassifier::new();

        let result = classifier.classify("小明数学成绩怎么样");
        assert_eq!(result.entities.subject, Some("数学".to_string()));
    }
}
