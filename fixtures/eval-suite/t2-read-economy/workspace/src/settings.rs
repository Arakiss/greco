pub const ACTIVE_LIMIT: usize = 32;
pub const RETRY_LIMIT: usize = 3;

pub fn accepts_batch(size: usize) -> bool {
    size <= ACTIVE_LIMIT
}
