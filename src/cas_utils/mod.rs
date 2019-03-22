#[derive(PartialEq, Copy, Clone)]
pub enum Status {
    Undecided,
    Failed,
    Successful,
}

impl From<usize> for Status {
    fn from(num: usize) -> Status {
        match num {
            0 => Status::Undecided,
            1 => Status::Failed,
            2 => Status::Successful,
            _ => panic!() // TODO: better way
        }
    }
}

impl Into<usize> for Status {
    fn into(self) -> usize {
        self as usize
    }
}

pub mod c_cas;
pub mod m_cas;
