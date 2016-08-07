pub mod multiboot;

use self::multiboot::MultibootTags;

#[no_mangle]
pub unsafe extern fn kstart(multiboot_tags: &MultibootTags) {
    let multiboot_info = multiboot_tags.parse();
    println!("{:?}", multiboot_info);
}
