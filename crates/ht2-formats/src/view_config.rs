//! Five-direction panel layout settings read by `0x4dfa..` via `0x507fa0`.
#[derive(Debug,PartialEq,Eq)]
pub struct ViewTables(pub [[i32;5];4]);
impl ViewTables {
    pub const KEYS:[&'static str;4]=["maskXview0","maskYview0","maskYview1","maskYtile"];
    pub fn from_values(values:[Option<&str>;4])->Self {
        let mut tables=[[0;5],[10;5],[380;5],[430;5]];
        for (table,value) in tables.iter_mut().zip(values){
            let Some(value)=value else{continue};
            let value=&value.as_bytes()[..value.len().min(99)];
            if value==b"off" {continue;}
            for (slot,token) in table.iter_mut().zip(value.split(|c|c.is_ascii_whitespace()).filter(|s|!s.is_empty())) {
                if token[0]==b'#' {continue;}
                // The original reports malformed long words. Layout loading
                // retains its numeric prefix; diagnostics are not reproduced here.
                let mut i=usize::from(matches!(token[0],b'+'|b'-'));let start=i;let mut n=0i64;
                while i<token.len() && token[i].is_ascii_digit(){n=(n*10+(token[i]-b'0') as i64).min(2147483648);i+=1;}
                if i==start {*slot=0;}else{if token[0]==b'-'{n=-n;}*slot=n.clamp(i32::MIN as i64,i32::MAX as i64) as i32;}
            }
        }
        Self(tables)
    }
    /// CAR section only; existing settings are read without modifying TRUCK.INI.
    pub fn from_ini(ini:&str)->Self {
        let mut car=false;let mut values=[None;4];
        for line in ini.lines(){
            let line=line.trim();if line.starts_with(';')||line.is_empty(){continue;}
            if line.starts_with('['){car=line.strip_prefix('[').and_then(|s|s.strip_suffix(']')).is_some_and(|s|s.eq_ignore_ascii_case("CAR"));continue;}
            if !car {continue;}
            if let Some((key,value))=line.split_once('='){
                if let Some(i)=Self::KEYS.iter().position(|k|k.eq_ignore_ascii_case(key.trim())){
                    if values[i].is_none(){values[i]=Some(value.trim().trim_matches('"'));}
                }
            }
        }
        Self::from_values(values)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_integer_layout_lists(){
        for line in include_str!("../../../tools/emu/golden_view_tables.tsv").lines(){
            let p:Vec<_>=line.split('\t').collect();let values=core::array::from_fn(|i|if p[i]=="<missing>"{None}else{Some(p[i])});
            let actual=ViewTables::from_values(values).0.into_iter().flatten().collect::<Vec<_>>();
            assert_eq!(actual,p[4].split(',').map(|s|s.parse::<i32>().unwrap()).collect::<Vec<_>>());
            let ini=SelfContained::ini(values);
            assert_eq!(ViewTables::from_ini(&ini).0.into_iter().flatten().collect::<Vec<_>>(),actual);
        }
    }
    struct SelfContained;
    impl SelfContained {fn ini(values:[Option<&str>;4])->String {
        let mut s=String::from("[OTHER]\nmaskXview0=999\n[car]\n");
        for (key,value) in ViewTables::KEYS.into_iter().zip(values){if let Some(v)=value{s.push_str(&format!("{key}={v}\n"));}}
        s
    }}
}
