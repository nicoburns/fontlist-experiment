/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

use font_types::NameId;
use memmap2::Mmap;
use skrifa::raw::{FileRef, TableProvider};
use std::time::Instant;

fn main() {
    println!("Enumerating fonts using CoreText...");

    let mut name = String::with_capacity(100);

    for_each_system_font(|ident| {
        let start = Instant::now();

        let file = std::fs::File::open(&ident.path).unwrap();
        let mapped = unsafe { Mmap::map(&file) }.unwrap();
        let font_file = FileRef::new(&mapped).unwrap();

        let index = match font_file {
            FileRef::Font(_) => 0,
            FileRef::Collection(collection) => 'idx: {
                for i in 0..collection.len() {
                    let font = collection.get(i).unwrap();
                    let name_table = font.name().unwrap();
                    if name_table
                        .name_record()
                        .iter()
                        .filter(|record| record.name_id() == NameId::POSTSCRIPT_NAME)
                        .any(|record| {
                            name.clear();
                            record
                                .string(name_table.string_data())
                                .unwrap()
                                .chars()
                                .for_each(|c| name.push(c));
                            &name == &ident.postscript_name
                        })
                    {
                        break 'idx i;
                    }
                }

                panic!(
                    "Font with postscript_name {} not found in collection",
                    ident.postscript_name
                );
            }
        };

        let time = Instant::now().duration_since(start).as_micros();

        println!(
            "{:>40} {} {} ({}us)",
            ident.postscript_name, index, ident.path, time
        );
    });
}

/// An identifier for a local font on a MacOS system. These values comes from the CoreText
/// CTFontCollection. Note that `path` here is required. We do not load fonts that do not
/// have paths.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FontIdentifier {
    pub postscript_name: String,
    pub path: String,
}

pub fn for_each_available_family(mut callback: impl FnMut(String)) {
    let family_names = core_text::font_collection::get_family_names();
    for family_name in family_names.iter() {
        callback(family_name.to_string());
    }
}

pub fn for_each_variation(family_name: &str, mut callback: impl FnMut(FontIdentifier)) {
    let family_collection = core_text::font_collection::create_for_family(family_name);
    if let Some(family_collection) = family_collection {
        if let Some(family_descriptors) = family_collection.get_descriptors() {
            for family_descriptor in family_descriptors.iter() {
                let path = family_descriptor.font_path();
                let Some(path) = path.as_ref().and_then(|path| path.to_str()) else {
                    continue;
                };

                let identifier = FontIdentifier {
                    postscript_name: String::from(family_descriptor.font_name()),
                    path: String::from(path),
                };
                callback(identifier);
            }
        }
    }
}

pub fn for_each_system_font(mut callback: impl FnMut(FontIdentifier)) {
    for_each_available_family(|family_name| for_each_variation(&family_name, &mut callback));
}
