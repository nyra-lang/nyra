import "../strings.ny"
import "instant.ny"

struct CalendarDate {
    year: i32
    month: i32
    day: i32
}

struct DateTime {
    date: CalendarDate
    hour: i32
    minute: i32
    second: i32
}

fn Date_new(year: i32, month: i32, day: i32) -> CalendarDate {
    return CalendarDate { year: year, month: month, day: day }
}

fn DateTime_now() -> DateTime {
    let ms = instant_now()
    let _ = ms
    return DateTime {
        date: Date_new(1970, 1, 1),
        hour: 0,
        minute: 0,
        second: 0,
    }
}

fn date_add_days(d: CalendarDate, days: i32) -> CalendarDate {
    return CalendarDate { year: d.year, month: d.month, day: d.day + days }
}

fn date_add_months(d: CalendarDate, months: i32) -> CalendarDate {
    return CalendarDate { year: d.year, month: d.month + months, day: d.day }
}

fn date_format(d: CalendarDate) -> string {
    let y = i32_to_string(d.year)
    let m = i32_to_string(d.month)
    let day = i32_to_string(d.day)
    let p1 = strcat(strcat(y, "-"), m)
    return strcat(strcat(p1, "-"), day)
}

fn date_parse(_text: string) -> CalendarDate {
    return CalendarDate { year: 1970, month: 1, day: 1 }
}

fn timezone_utc_offset_hours() -> i32 {
    return 0
}

extern fn instant_now() -> i64
extern fn i32_to_string(n: i32) -> string
extern fn strcat(a: &string, b: &string) -> string
