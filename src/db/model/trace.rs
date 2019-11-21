use std::fs::File;
use std::io::Write;

use diesel::*;
use rayon::prelude::*;
use uuid::Uuid;

#[derive(Queryable, Debug)]
pub struct Trace {
    pub id: i32,
    pub process: String,
    pub function_list: Vec<String>,
    pub environment: Vec<String>,
    pub values: Vec<String>,
    pub options: Vec<String>,
}

macro_rules! template {
    ("STAP") => {
r#"
probe process("{}").function("{}").call {{
    printf("probe: %s", ppfunc());
    print_usyms(ucallers(5));
}}
"#
};
    ("BPF") => {
    r#"uprobe:{}:{} {{ printf("probe: %s\n%s\n", probe, ustack(perf, 5)); }}"#
    };
}

fn ending(s: &str, t: usize) -> String {
    match s {
        "STAP" =>
            format!("probe timer.s({}) {{exit(); }}\n", t),
        "BPF" =>
            format!("interval:s:({}) {{ exit(); }}\n", t),
        _ => unreachable!()
    }
}


impl Trace {
    pub fn to_content_stap(&self) -> String {
        self.function_list.par_iter().map(|x| {
            format!(template!("STAP"), self.process, x)
        }).reduce_with(|mut x, y| {
            x.push_str(y.as_str());
            x
        }).unwrap()
    }
    pub fn to_content_bpf(&self) -> String {
        self.function_list.par_iter().map(|x| {
            format!(template!("BPF"), self.process, x)
        }).reduce_with(|mut x, y| {
            x.push_str(y.as_str());
            x
        }).unwrap()
    }
    pub fn to_file_stap(&self, duration: usize) -> std::io::Result<String> {
        let content = self.to_content_stap();
        let name = format!("/tmp/{}.stap", Uuid::new_v4());
        let mut file = File::create(name.as_str())?;
        file.write(content.as_bytes())?;
        file.write(ending("STAP", duration).as_bytes())?;
        file.flush()?;
        Ok(name)
    }
    pub fn to_file_bpf(&self, duration: usize) -> std::io::Result<String> {
        let content = self.to_content_bpf();
        let name = format!("/tmp/{}.bpf", Uuid::new_v4());
        let mut file = File::create(name.as_str())?;
        file.write(content.as_bytes())?;
        file.write(ending("BPF", duration).as_bytes())?;
        file.flush()?;
        Ok(name)
    }
}


mod test {
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_to_content() {
        use crate::db::schema::trace::traces::dsl::*;
        use diesel::prelude::*;
        use super::Trace;
        let conn = crate::db::connection::get_conn();
        let res = traces.load::<Trace>(&*conn).unwrap();
        let mut file = File::create("/tmp/cargo_test").unwrap();
        for i in res {
            writeln!(file, "{}", i.to_content_stap()).unwrap();
        }
    }

    #[test]
    fn test_to_file() {
        use crate::db::schema::trace::traces::dsl::*;
        use diesel::prelude::*;
        use super::Trace;
        let conn = crate::db::connection::get_conn();
        let res = traces.load::<Trace>(&*conn).unwrap();
        for i in res {
            i.to_file_stap(5).unwrap();
        }
    }
}