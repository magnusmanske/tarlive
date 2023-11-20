# ZIPLIVE

## The problem
- You have a bunch of files
- You want to `tar` them for transfer, but ideally not in an actual file (eg because of disk usage)
- You transfer the data but something terminates the transfer prematurely
- You have the file size of the received tar file
- You want to continue where you left off


## The solution
If you use `tarlive`, you can do just that! `tarlive` creates a metadata file that logs where files end in the `tar` archive.
If you call `tarlive` with `--offset NUMBER` (the size of the received part of the file, plus 1), it will skip all completely transferred files,
and start generating the `tar` file from the first partially transferred file.
The output can simply be appended to the partially transferred file.
The result will be binary-identical to a single successful file transfer.

## Example
With `input.files` being a file containing the list of files to transfer, you can create a complete `tar` archive with:
```bash
ziplive --input input.files --tar output.tar
```

You can "continue" a partial archive (eg `output.partial.tar`) of size 123456 bytes with
```bash
ziplive --input input.files --tar output.remaining.tar --offset 123457
```
(note that the offset is the partial file size plus one!)

You can just concatenate them to a full archive:
```bash
cat output.partial.tar output.remaining.tar > output.tar
```

## Notes
- For each `input.files` file, a metadata file will be created
- This file is written to the current `tmp` directory
- Its name is a `base64` hash of the names, sizes, and last modification date of the individual files, and ends with `.ziplive`
- You can delete these `.ziplive` files (ideally when `ziplive` is not running), but that will remove the option of continuing a partial archive at an `--offset` without reading all files (the output will start at the correct `--offset`, it will just take longer to start)
