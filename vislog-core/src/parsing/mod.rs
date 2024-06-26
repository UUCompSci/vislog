use std::str::FromStr;

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer,
};
use serde_json::Value;

use crate::{
    Course, CourseDetails, CourseEntries, CourseEntry, Label, Requirement, RequirementModule,
    Requirements,
};

use self::{
    courses::{parse_course_credits, CoursesParser, RawCourseEntry},
    guid::Guid,
};

pub mod courses;
pub mod guid;

impl<'de> Deserialize<'de> for Requirements {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RequirementsVisitor;

        impl<'de> Visitor<'de> for RequirementsVisitor {
            type Value = Requirements;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a JSON object representing a `RequirementModule` or a JSON array of `RequirementModule`s")
            }

            /// Case for [Requirements::Single] variant
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                /// Intermediate struct used to determine if `requirement_list` is a JSON object or array.
                #[derive(Debug, Deserialize)]
                #[serde(untagged)]
                enum RawRequirement {
                    /// Case where the `RequirementModule` only has a single `Course` JSON object in field
                    /// `course`
                    SingleCourseRequirement(SingleCourseRequirement),
                    Single(Requirement),
                    Many(Vec<Requirement>),
                }

                #[derive(Debug, Deserialize)]
                struct SingleCourseRequirement {
                    title: Option<String>,
                    course: Course,
                }

                let mut title: Option<Option<String>> = None;
                let mut req_narrative: Option<Option<String>> = None;
                let mut requirement_list: Option<RawRequirement> = None;

                while let Ok(Some(key)) = map.next_key::<String>() {
                    match key.as_str() {
                        "title" => {
                            if title.is_some() {
                                return Err(de::Error::duplicate_field("title"));
                            }
                            title = Some(map.next_value()?);
                        }
                        "req_narrative" => {
                            if req_narrative.is_some() {
                                return Err(de::Error::duplicate_field("req_narrative"));
                            }
                            req_narrative = Some(map.next_value()?);
                        }
                        "requirement_list" => {
                            if requirement_list.is_some() {
                                return Err(de::Error::duplicate_field("requirement_list"));
                            }
                            requirement_list = Some(map.next_value()?);
                        }
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>();
                        }
                    }
                }

                let title = title.ok_or_else(|| de::Error::missing_field("title"))?;

                let requirements = requirement_list
                    .ok_or_else(|| de::Error::missing_field("requirements_list"))?;

                let requirement_module = match requirements {
                    RawRequirement::Single(requirement) => {
                        RequirementModule::SingleBasicRequirement { title, requirement }
                    }
                    RawRequirement::Many(requirements) => RequirementModule::BasicRequirements {
                        title,
                        requirements,
                    },
                    RawRequirement::SingleCourseRequirement(SingleCourseRequirement {
                        title: req_title,
                        course,
                    }) => {
                        let requirement = Requirement::Courses {
                            title: req_title,
                            courses: CourseEntries(vec![CourseEntry::Course(course)]),
                        };
                        RequirementModule::SingleBasicRequirement { title, requirement }
                    }
                };

                Ok(Requirements::Single(requirement_module))
            }

            /// Case for [Requirements::Many] variant
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut modules = Vec::new();
                while let Ok(Some(module)) = seq.next_element() {
                    modules.push(module);
                }

                Ok(Requirements::Many(modules))
            }
        }

        deserializer.deserialize_any(RequirementsVisitor)
    }
}

impl<'de> Deserialize<'de> for RequirementModule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RequirementModuleVisitor;

        impl<'de> Visitor<'de> for RequirementModuleVisitor {
            type Value = RequirementModule;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                // TODO: Improve this message
                formatter.write_str("a JSON object representing a program at Union University")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut title: Option<Option<String>> = None;
                let mut requirements: Option<Vec<Requirement>> = None;

                while let Ok(Some(key)) = map.next_key::<String>() {
                    match key.as_str() {
                        "title" => {
                            if title.is_some() {
                                return Err(de::Error::duplicate_field("title"));
                            }
                            title = Some(map.next_value()?);
                        }
                        "requirement_list" => {
                            if requirements.is_some() {
                                return Err(de::Error::duplicate_field("requirement_list"));
                            }
                            requirements = Some(map.next_value()?);
                        }
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>();
                        }
                    }
                }

                let title = title.ok_or_else(|| de::Error::missing_field("title"))?;
                let requirements =
                    requirements.ok_or_else(|| de::Error::missing_field("requirements"))?;

                Ok(RequirementModule::BasicRequirements {
                    title,
                    requirements,
                })
            }
        }

        deserializer.deserialize_any(RequirementModuleVisitor)
    }
}

impl<'de> Deserialize<'de> for Requirement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RequirementVisitor;

        impl<'de> Visitor<'de> for RequirementVisitor {
            type Value = Requirement;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a JSON object representing a `Requirement` enum")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut title: Option<Option<String>> = None;
                let mut req_narrative: Option<Option<String>> = None;
                let mut courses = None;

                while let Ok(Some(key)) = map.next_key::<String>() {
                    match key.as_str() {
                        "title" => {
                            if title.is_some() {
                                return Err(de::Error::duplicate_field("title"));
                            }

                            title = Some(map.next_value()?);
                        }
                        "req_narrative" => {
                            if req_narrative.is_some() {
                                return Err(de::Error::duplicate_field("req_narrative"));
                            }

                            req_narrative = Some(map.next_value()?);
                        }
                        "course" => {
                            if courses.is_some() {
                                return Err(de::Error::duplicate_field("course"));
                            }

                            courses = Some(map.next_value()?);
                        }
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>();
                        }
                    }
                }

                let title = title.ok_or_else(|| de::Error::missing_field("title"))?;
                let req_narrative =
                    req_narrative.ok_or_else(|| de::Error::missing_field("req_narrative"))?;

                let requirement = match (title, courses) {
                    (Some(title), courses) if title.contains("Select") => {
                        Requirement::SelectFromCourses { title, courses }
                    }
                    (title, Some(course_entries)) => Requirement::Courses {
                        title,
                        courses: course_entries,
                    },
                    (title, None) => Requirement::Label {
                        title,
                        req_narrative,
                    },
                };

                Ok(requirement)
            }
        }

        deserializer.deserialize_any(RequirementVisitor)
    }
}

impl<'de> Deserialize<'de> for CourseEntries {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CourseEntriesVisitor;

        impl<'de> Visitor<'de> for CourseEntriesVisitor {
            type Value = CourseEntries;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an array of JSON objects representing a `SelectionEntry`")
            }

            // Normal code path for `Requirement`s with a JSON array of `Course` objects in `course` field
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut raw_entries = Vec::with_capacity(seq.size_hint().unwrap_or(4));

                while let Ok(Some(raw_entry)) = seq.next_element::<RawCourseEntry>() {
                    raw_entries.push(raw_entry)
                }

                let course_entries = CoursesParser::new(raw_entries)
                    .parse()
                    .map_err(de::Error::custom)?;

                Ok(course_entries)
            }

            // Code path for `Requirement`s that have a single JSON object in `course` field
            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut url: Option<String> = None;
                let mut path: Option<String> = None;
                let mut guid: Option<Guid> = None;
                let mut name: Option<Option<String>> = None;
                let mut number: Option<Option<String>> = None;
                let mut subject_name: Option<Option<String>> = None;
                let mut subject_code: Option<Option<String>> = None;
                let mut credits: Option<(u8, Option<u8>)> = None;
                let mut is_narrative: Option<bool> = None;

                while let Ok(Some(key)) = map.next_key::<String>() {
                    match key.as_str() {
                        "url" => {
                            if url.is_some() {
                                return Err(de::Error::duplicate_field("url"));
                            }

                            url = Some(map.next_value()?);
                        }
                        "path" => {
                            if path.is_some() {
                                return Err(de::Error::duplicate_field("path"));
                            }

                            path = Some(map.next_value()?);
                        }
                        "guid" => {
                            if guid.is_some() {
                                return Err(de::Error::duplicate_field("guid"));
                            }

                            let guid_str_with_braces = map.next_value::<&str>()?;

                            let guid_str_trimmed =
                                &guid_str_with_braces[1..guid_str_with_braces.len() - 1];

                            guid = Some(Guid::try_from(guid_str_trimmed).map_err(|e| {
                                de::Error::custom(format!("error parsing guid: {}", e))
                            })?);
                        }
                        "name" => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }

                            name = Some(map.next_value()?);
                        }
                        "number" => {
                            if number.is_some() {
                                return Err(de::Error::duplicate_field("number"));
                            }

                            number = Some(map.next_value()?);
                        }
                        "subject_name" => {
                            if subject_name.is_some() {
                                return Err(de::Error::duplicate_field("subject_name"));
                            }

                            subject_name = Some(map.next_value()?);
                        }
                        "subject_code" => {
                            if subject_code.is_some() {
                                return Err(de::Error::duplicate_field("subject_code"));
                            }

                            subject_code = Some(map.next_value()?);
                        }
                        "credits" => {
                            if credits.is_some() {
                                return Err(de::Error::duplicate_field("credits"));
                            }

                            let credits_str = map.next_value::<&str>()?;
                            credits =
                                Some(parse_course_credits(credits_str).map_err(de::Error::custom)?);
                        }
                        "is_narrative" => {
                            if is_narrative.is_some() {
                                return Err(de::Error::duplicate_field("is_narrative"));
                            }

                            let is_narrative_str = map.next_value::<&str>()?;

                            is_narrative = Some(match is_narrative_str {
                                "True" => true,
                                "False" => false,
                                invalid_str => {
                                    return Err(de::Error::custom(format!(
                                        r#"Expected "True" or "False". Got: {}"#,
                                        invalid_str
                                    )))
                                }
                            });
                        }
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>();
                        }
                    }
                }

                let url = url.ok_or_else(|| de::Error::missing_field("url"))?;
                let path = path.ok_or_else(|| de::Error::missing_field("path"))?;
                let guid = guid.ok_or_else(|| de::Error::missing_field("guid"))?;
                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let number = number.ok_or_else(|| de::Error::missing_field("number"))?;
                let subject_name =
                    subject_name.ok_or_else(|| de::Error::missing_field("subject_name"))?;
                let subject_code =
                    subject_code.ok_or_else(|| de::Error::missing_field("subject_code"))?;
                let credits = credits.ok_or_else(|| de::Error::missing_field("credits"))?;
                let is_narrative =
                    is_narrative.ok_or_else(|| de::Error::missing_field("is_narrative"))?;

                let entry = if is_narrative {
                    let name = name.ok_or(de::Error::custom(
                        "`name` field for `Label` should not be null",
                    ))?;
                    CourseEntry::Label(Label {
                        url,
                        guid,
                        name,
                        subject_code,
                        credits,
                        number,
                    })
                } else {
                    let number = number.ok_or(de::Error::custom(
                        "`number` field for `Course` should not be null",
                    ))?;
                    let subject_code = subject_code.ok_or(de::Error::custom(
                        "`subject_code` field for `Course` should not be null",
                    ))?;
                    CourseEntry::Course(Course {
                        url,
                        path,
                        guid,
                        name,
                        number,
                        subject_name,
                        subject_code,
                        credits,
                    })
                };

                Ok(CourseEntries(vec![entry]))
            }
        }

        deserializer.deserialize_any(CourseEntriesVisitor)
    }
}

impl<'de> Deserialize<'de> for CourseDetails {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct CourseDetailsVisitor;

        impl<'de> Visitor<'de> for CourseDetailsVisitor {
            type Value = CourseDetails;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a JSON object representing a `CourseDetail` struct")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: de::MapAccess<'de>,
            {
                let mut url: Option<String> = None;
                let mut guid: Option<String> = None;
                let mut path: Option<String> = None;
                let mut subject_code: Option<String> = None;
                let mut subject_name: Option<Option<String>> = None;
                let mut number: Option<String> = None;
                let mut name: Option<String> = None;
                let mut credits_min: Option<Option<String>> = None;
                let mut credits_max: Option<Option<String>> = None;
                let mut description: Option<String> = None;
                let mut prerequisite_narrative: Option<Option<String>> = None;
                let mut prerequisite: Option<Value> = None;
                let mut corequisite_narrative: Option<Option<String>> = None;
                let mut corequisite: Option<Value> = None;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "url" => {
                            if url.is_some() {
                                return Err(de::Error::duplicate_field("url"));
                            }
                            url = Some(map.next_value()?);
                        }
                        "GUID" => {
                            if guid.is_some() {
                                return Err(de::Error::duplicate_field("guid"));
                            }
                            guid = Some(map.next_value()?);
                        }
                        "path" => {
                            if path.is_some() {
                                return Err(de::Error::duplicate_field("path"));
                            }
                            path = Some(map.next_value()?);
                        }
                        "subject_code" => {
                            if subject_code.is_some() {
                                return Err(de::Error::duplicate_field("subject_code"));
                            }
                            subject_code = Some(map.next_value()?);
                        }
                        "subject_name" => {
                            if subject_name.is_some() {
                                return Err(de::Error::duplicate_field("subject_name"));
                            }
                            subject_name = Some(map.next_value()?);
                        }
                        "number" => {
                            if number.is_some() {
                                return Err(de::Error::duplicate_field("number"));
                            }
                            number = Some(map.next_value()?);
                        }
                        "name" => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        }
                        "credits_min" => {
                            if credits_min.is_some() {
                                return Err(de::Error::duplicate_field("credits_min"));
                            }
                            credits_min = Some(map.next_value()?);
                        }
                        "credits_max" => {
                            if credits_max.is_some() {
                                return Err(de::Error::duplicate_field("credits_max"));
                            }
                            credits_max = Some(map.next_value()?);
                        }
                        "description" => {
                            if description.is_some() {
                                return Err(de::Error::duplicate_field("description"));
                            }
                            description = Some(map.next_value()?);
                        }
                        "prerequisite_narrative" => {
                            if prerequisite_narrative.is_some() {
                                return Err(de::Error::duplicate_field("prerequisite_narrative"));
                            }
                            prerequisite_narrative = Some(map.next_value()?);
                        }
                        "prerequisite" => {
                            if prerequisite.is_some() {
                                return Err(de::Error::duplicate_field("prerequisite"));
                            }
                            prerequisite = Some(map.next_value()?);
                        }
                        "corequisite_narrative" => {
                            if corequisite_narrative.is_some() {
                                return Err(de::Error::duplicate_field("corequisite_narrative"));
                            }
                            corequisite_narrative = Some(map.next_value()?);
                        }
                        "corequisite" => {
                            if corequisite.is_some() {
                                return Err(de::Error::duplicate_field("corequisite"));
                            }
                            corequisite = Some(map.next_value()?);
                        }
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>();
                        }
                    }
                }

                let url = url.ok_or(de::Error::missing_field("url"))?;
                let path = path.ok_or(de::Error::missing_field("path"))?;
                let subject_code = subject_code.ok_or(de::Error::missing_field("subject_code"))?;
                let subject_name = subject_name.ok_or(de::Error::missing_field("subject_name"))?;
                let number = number.ok_or(de::Error::missing_field("number"))?;
                let name = name.ok_or(de::Error::missing_field("name"))?;
                let description = description.ok_or(de::Error::missing_field("description"))?;
                let prerequisite_narrative = prerequisite_narrative
                    .ok_or(de::Error::missing_field("prerequisite_narrative"))?;
                let corequisite_narrative = corequisite_narrative
                    .ok_or(de::Error::missing_field("corequisite_narrative"))?;

                // Transform into integers
                let credits_min = {
                    let float_str = credits_min.ok_or(de::Error::missing_field("credits_min"))?;

                    // NOTE: Assume credits equal zero when `credits_min` is `null` in JSON format
                    if let Some(float_str) = float_str {
                        let float: f32 = float_str.parse().map_err(|e| de::Error::custom(e))?;
                        if float > u8::MAX as f32 {
                            return Err(de::Error::custom(format!(
                                "value of credits_max exceeded `u8::MAX` (255)"
                            )));
                        }
                        float.trunc() as u8
                    } else {
                        0
                    }
                };

                let credits_max = {
                    let float_option =
                        credits_max.ok_or(de::Error::missing_field("credits_max"))?;

                    float_option
                        .map(|float_str| float_str.parse::<f32>().map_err(|e| de::Error::custom(e)))
                        .transpose()?
                        .map(|float| {
                            if float <= u8::MAX as f32 {
                                Ok(float.trunc() as u8)
                            } else {
                                Err(de::Error::custom(format!(
                                    "value of credits_max exceeded 255"
                                )))
                            }
                        })
                        .transpose()?
                };

                // These are optional fields
                let prerequisite = prerequisite
                    .map(|v| extract_guid_from_requisite(v).map_err(|e| de::Error::custom(e)))
                    .transpose()?;
                let corequisite = corequisite
                    .map(|v| extract_guid_from_requisite(v).map_err(|e| de::Error::custom(e)))
                    .transpose()?;

                let guid_str = guid.ok_or(de::Error::missing_field("GUID"))?;
                let guid = Guid::try_from(&guid_str[1..guid_str.len() - 1])
                    .map_err(|e| de::Error::custom(e))?;

                // Construct CourseDetails
                let course_details = CourseDetails {
                    url,
                    guid,
                    path,
                    subject_code,
                    subject_name,
                    number,
                    name,
                    credits_min,
                    credits_max,
                    description,
                    prerequisite_narrative,
                    prerequisite,
                    corequisite_narrative,
                    corequisite,
                };

                Ok(course_details)
            }
        }

        /// Extracts only the `GUID` field from a [Value](serde_json::Value) constructed from
        /// the `prerequisite` or `corequisite` field of an unparsed JSON object representing
        /// the [CourseDetails](crate::CourseDetails) struct
        fn extract_guid_from_requisite(requisite_json: Value) -> Result<Guid, String> {
            let Value::Object(map) = requisite_json else {
                return Err("expected JSON object".to_owned());
            };

            let guid_str = map.get("GUID").ok_or("missing field GUID")?;
            let Value::String(guid_str) = guid_str else {
                return Err("expected JSON string for field GUID".to_owned());
            };

            let guid_str_without_curly_braces = &guid_str[1..guid_str.len() - 1];

            Guid::try_from(guid_str_without_curly_braces).map_err(|e| e.to_string())
        }

        deserializer.deserialize_map(CourseDetailsVisitor)
    }
}

pub(crate) fn deserialize_from_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(|e| de::Error::custom(e))
}

pub(crate) fn deserialize_and_floor_u8_from_float_str<'de, D>(
    deserializer: D,
) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let float: f32 = s
        .parse()
        .map_err(|e| de::Error::custom(format!("failed to parse f32, {e}")))?;
    if float > 255.0 {
        Err(de::Error::custom(format!(
            "expected a value less than '255.0', instead got: {float}"
        )))
    } else {
        Ok(float.trunc() as u8)
    }
}

pub(crate) fn deserialize_extract_guid_only<'de, D>(
    deserializer: D,
) -> Result<Option<Guid>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ExtractGuidVisitor;

    impl<'d> Visitor<'d> for ExtractGuidVisitor {
        type Value = Option<Guid>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string representing a GUID surounded by curly braces")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'d>,
        {
            let mut guid: Option<String> = None;

            while let Ok(Some(key)) = map.next_key::<&str>() {
                match key {
                    "GUID" => {
                        guid = map.next_value()?;
                        break;
                    }
                    _ => {
                        let _ = map.next_value::<de::IgnoredAny>();
                    }
                }
            }

            match guid {
                Some(s) if s.len() < 32 => {
                    Err(de::Error::custom("string not long enough to be GUID"))
                }
                Some(s) => Ok(Some(
                    Guid::try_from(&s[1..s.len() - 1]).map_err(|e| de::Error::custom(e))?,
                )),
                None => Ok(None),
            }
        }
    }

    deserializer.deserialize_map(ExtractGuidVisitor)
}
