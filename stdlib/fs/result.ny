import "file.ny"
import "../error.ny"

fn read_text(path: string) -> Result<string, Error> {
    if file_exists(path) == 0 {
        return Result.Err(Error_io(strcat("file not found: ", path)))
    }
    return Result.Ok(read_file(path))
}

fn read_text_limit(path: string, max_bytes: i32) -> Result<string, Error> {
    if file_exists(path) == 0 {
        return Result.Err(Error_io(strcat("file not found: ", path)))
    }
    return Result.Ok(read_file_limit(path, max_bytes))
}

fn write_text(path: string, content: string) -> Result<i32, Error> {
    let status = write_file(path, content)
    if status != 0 {
        return Result.Err(Error_io(strcat("write failed: ", path)))
    }
    return Result.Ok(status)
}

fn append_text(path: string, content: string) -> Result<i32, Error> {
    let status = append_file(path, content)
    if status != 0 {
        return Result.Err(Error_io(strcat("append failed: ", path)))
    }
    return Result.Ok(status)
}
