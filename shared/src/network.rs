#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Ticket {
    pub plate: String,
    pub road: u16,
    pub mile1: u16,
    pub timestamp1: u32,
    pub mile2: u16,
    pub timestamp2: u32,
    pub speed: u16,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Camera {
    pub road: u16,
    pub mile: u16,
    pub limit: u16,
}
