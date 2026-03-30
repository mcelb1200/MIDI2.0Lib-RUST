#[cfg(test)]
mod tests {
    use super::super::utils::{scale_down, scale_up};

    #[test]
    fn test_scale_up_no_panic() {
        let _ = scale_up(0, 33, 8);
        let _ = scale_up(0, 64, 8);
        let _ = scale_up(1, 64, 8);
        let _ = scale_up(1, 8, 64);
        let _ = scale_up(1, 64, 64);
        let _ = scale_up(1, 33, 33);
        let _ = scale_up(1, 40, 40);
        let _ = scale_up(1, 100, 100);
        let _ = scale_up(1, 200, 200);
    }

    #[test]
    fn test_scale_down_no_panic() {
        let _ = scale_down(100, 40, 8);
        let _ = scale_down(100, 64, 8);
        let _ = scale_down(100, 100, 8);
    }
}
