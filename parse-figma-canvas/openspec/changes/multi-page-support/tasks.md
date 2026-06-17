## 1. Core Functions

- [x] 1.1 Add `get_all_canvases(data)` function that returns `Vec<&Value>` of all pages
- [x] 1.2 Add `sanitize_folder_name(name)` function that converts page name to valid folder name
- [x] 1.3 Add `get_page_description(page)` function that extracts page description if any
- [x] 1.4 Add `find_layer_across_pages(data, name)` function that searches all pages
- [x] 1.5 Add `find_node_across_pages(data, name)` function that searches all pages

## 2. Output Structure

- [x] 2.1 Modify `execute_to_file()` to accept canvas parameter and write to page subfolder
- [x] 2.2 Modify `run_all()` to loop over all pages and create per-page subfolders
- [x] 2.3 Add `generate_index_md(pages, output_dir)` function to create index.md
- [x] 2.4 Add folder name conflict detection and numeric suffix handling

## 3. Command Modifications

- [x] 3.1 Modify `cmd_tree()` to accept canvas parameter instead of data
- [x] 3.2 Modify `cmd_texts()` to accept canvas parameter instead of data
- [x] 3.3 Modify `cmd_images()` to accept canvas parameter instead of data
- [x] 3.4 Modify `cmd_interactions()` to accept canvas parameter instead of data
- [x] 3.5 Modify `cmd_tokens()` to accept canvas parameter instead of data
- [x] 3.6 Modify `cmd_layers()` to accept canvas parameter instead of data

## 4. Error Handling

- [x] 4.1 Add `print_error(msg)` function for error JSON output
- [x] 4.2 Add `print_success(pages)` function for success JSON output
- [x] 4.3 Modify `main()` to handle file read errors with JSON output
- [x] 4.4 Modify `main()` to handle JSON parse errors with JSON output
- [x] 4.5 Modify `main()` to handle layer not found errors with JSON output
- [x] 4.6 Modify `main()` to handle node not found errors with JSON output

## 5. Main Loop Refactor

- [x] 5.1 Modify `main()` to use `get_all_canvases()` instead of `get_canvas()`
- [x] 5.2 Modify `main()` to create per-page subfolders for all commands
- [x] 5.3 Modify `main()` to generate index.md after processing
- [x] 5.4 Modify `main()` to print JSON summary to stdout
- [x] 5.5 Modify `main()` to redirect progress messages to stderr

## 6. Testing

- [x] 6.1 Add unit test for `sanitize_folder_name()` with various inputs
- [x] 6.2 Add unit test for `get_all_canvases()` with multi-page document
- [x] 6.3 Add unit test for `find_layer_across_pages()` with layer in different pages
- [x] 6.4 Add unit test for `find_node_across_pages()` with node in different pages
- [x] 6.5 Add integration test for multi-page processing with `all` command
- [x] 6.6 Add integration test for error handling with invalid input
