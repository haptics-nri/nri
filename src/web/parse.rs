use std::io::BufRead;
use super::flow::{Flow, FlowState, FlowCmd};
use ::teensy::ParkState;

pub fn parse<R: BufRead>(reader: R) -> Result<Flow, (u32, &'static str)> {
    let lines = try!(reader.lines()
                           .collect::<Result<Vec<_>,_>>()
                           .map_err(|_| (0, "I/O error")));
    let mut lines = lines.into_iter()
                           .map(|s| s.trim_right().to_owned())
                           .peekable();
                      
    let name = try!(lines.find(|s| s.len() > 0).ok_or((0, "Empty file")));
    let mut states = vec![];
    
    let mut i = 0;
    while lines.peek().is_some() {
        i += 1;
        let header = lines.next().unwrap();
        
        if header.len() == 0 { continue; }
        let header = header.split("=>").collect::<Vec<_>>();
        if header.len() > 2 { return Err((i, "too many arrows")); }
        
        match header[0].chars().next() {
            Some('-') => {},
            Some(' ') => return Err((i, "command outside of state")),
            _ => return Err((i, "unindented line")),
        }
        
        let (name, park) = match header.len() {
            1 => { (header[0][1..].trim().to_owned(), None) },
            2 => { (header[1].trim().to_owned(),
                    Some(match header[0][1..].trim() {
                            ""          => ParkState::None,
                            "BioTac"    => ParkState::BioTac,
                            "OptoForce" => ParkState::OptoForce,
                            "Stick"     => ParkState::Stick,
                            _           => return Err((i, "bad trigger")),
                        })
                    )
                  },
            _ => unreachable!(),
        };
        
        let mut script = vec![];
        
        while lines.peek().is_some() && lines.peek().unwrap().starts_with(' ') {
            i += 1;
            let line = lines.next().unwrap();
            let line = line.trim();
            
            if        line.starts_with(':') {
                script.push(FlowCmd::Send(line[1..].trim().to_owned()));
            } else if line.starts_with('"') {
                script.push(FlowCmd::Message(line[1..line.len()-1].to_owned()));
            } else if line.starts_with('>') {
                let nquotes = line.replace("\\\"", "").matches('"').count();
                if nquotes == 1 { return Err((i, "unterminated string")); }
                if nquotes > 2 { return Err((i, "too many strings")); }
                
                let s = line[line.find('"').unwrap()+1 .. line.rfind('"').unwrap()].trim();
                let range = line[line.rfind('"').unwrap()+1 ..].trim();
                
                if range.len() > 0 {
                    if !(   range.chars().next() == Some('(')
                         && range.chars().rev().next() == Some(')')) {
                        return Err((i, "range not in parentheses"));
                    }
                    let dots = try!(range.find("..").ok_or((i, "not enough dots in range")));
                    let low = try!(range[1..dots].parse().map_err(|_| (i, "bad start of range")));
                    let high = try!(range[dots+2..range.len()-1].parse().map_err(|_| (i, "bad end of range")));
                    
                    script.push(FlowCmd::int(s.to_owned(), (low, high)));
                } else {
                    script.push(FlowCmd::str(s.to_owned()));
                }
            } else {
                let mut words = line.split(' ').map(str::trim);
                match words.next() {
                    None | Some("") => return Err((i, "empty command")),
                    Some("stop") => {
                        let mut words = words.peekable();
                        if words.peek().is_some() {
                            for word in words {
                                script.push(FlowCmd::Stop(word.to_owned()));
                            }
                        } else {
                            script.push(FlowCmd::StopSensors);
                        }
                    },
                    Some("start") => {
                        for word in words {
                            script.push(FlowCmd::Start(word.to_owned()));
                        }
                    },
                    Some(_) => return Err((i, "invalid command")),
                }
            }
        }
        
        states.push(FlowState::new(name, park, script));
    }
    
    Ok(Flow::new(name, states))
}

