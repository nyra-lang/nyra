// Time ergonomics — short Instant/Duration helpers.
import "instant.ny"

fn now() -> Instant {
    return Instant_now()
}

fn ms(n: i32) -> Duration {
    return Duration_from_ms(n)
}

impl Duration {
    fn sleep(self) -> void {
        sleep_ms(self.millis)
    }
}
