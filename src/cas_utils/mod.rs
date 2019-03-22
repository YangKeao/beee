#[derive(PartialEq)]
pub enum Status {
    Undecided,
    Failed,
    Successful,
}

pub mod c_cas;
pub mod m_cas;
