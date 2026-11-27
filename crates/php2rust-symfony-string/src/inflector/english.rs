//! Port of Symfony\Component\String\Inflector\EnglishInflector.php
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Suffix tables matched on the reversed, ASCII-lowercased word, byte by
//! byte, as upstream does with `strrev()` and `strtolower()`; a vowel is
//! one of `aeiou`; the case of the suffix's first letter is kept with
//! `ctype_upper()` and `ucfirst()` (ASCII).
//!
//! Upstream: symfony/string Inflector/EnglishInflector.php

use super::Inflector;
use crate::exception::Result;

/// A table entry: the suffix reversed, its length, whether it may follow
/// a vowel, whether it may follow a consonant, and the replacement
/// suffixes (several when the form is ambiguous).
type Map = (&'static [u8], usize, bool, bool, &'static [&'static str]);

/// Map English plural to singular suffixes.
///
/// See <http://english-zone.com/spelling/plurals.html>
const PLURAL_MAP: &[Map] = &[
    // insignias (insigne), insignia (insigne)
    (b"saingisni", 9, true, true, &["insigne"]),
    (b"aingisni", 8, true, true, &["insigne"]),
    // passersby (passerby)
    (b"ybsressap", 9, true, true, &["passerby"]),
    // nodes (node)
    (b"sedon", 5, true, true, &["node"]),
    // bacteria (bacterium)
    (b"airetcab", 8, true, true, &["bacterium"]),
    // issues (issue)
    (b"seussi", 6, true, true, &["issue"]),
    // corpora (corpus)
    (b"aroproc", 7, true, true, &["corpus"]),
    // criteria (criterion)
    (b"airetirc", 8, true, true, &["criterion"]),
    // curricula (curriculum)
    (b"alucirruc", 9, true, true, &["curriculum"]),
    // quora (quorum)
    (b"arouq", 5, true, true, &["quorum"]),
    // genera (genus)
    (b"areneg", 6, true, true, &["genus"]),
    // media (medium)
    (b"aidem", 5, true, true, &["medium"]),
    // memoranda (memorandum)
    (b"adnaromem", 9, true, true, &["memorandum"]),
    // phenomena (phenomenon)
    (b"anemonehp", 9, true, true, &["phenomenon"]),
    // strata (stratum)
    (b"atarts", 6, true, true, &["stratum"]),
    // nebulae (nebula)
    (b"ea", 2, true, true, &["a"]),
    // services (service)
    (b"secivres", 8, true, true, &["service"]),
    // mice (mouse), lice (louse)
    (b"eci", 3, false, true, &["ouse"]),
    // geese (goose)
    (b"esee", 4, false, true, &["oose"]),
    // fungi (fungus), alumni (alumnus), syllabi (syllabus), radii (radius)
    (b"i", 1, true, true, &["us"]),
    // men (man), women (woman)
    (b"nem", 3, true, true, &["man"]),
    // children (child)
    (b"nerdlihc", 8, true, true, &["child"]),
    // oxen (ox)
    (b"nexo", 4, false, false, &["ox"]),
    // indices (index), appendices (appendix), prices (price)
    (b"seci", 4, false, true, &["ex", "ix", "ice"]),
    // codes (code)
    (b"sedoc", 5, false, true, &["code"]),
    // selfies (selfie)
    (b"seifles", 7, true, true, &["selfie"]),
    // zombies (zombie)
    (b"seibmoz", 7, true, true, &["zombie"]),
    // movies (movie)
    (b"seivom", 6, true, true, &["movie"]),
    // names (name)
    (b"seman", 5, true, false, &["name"]),
    // conspectuses (conspectus), prospectuses (prospectus)
    (b"sesutcep", 8, true, true, &["pectus"]),
    // feet (foot)
    (b"teef", 4, true, true, &["foot"]),
    // geese (goose)
    (b"eseeg", 5, true, true, &["goose"]),
    // teeth (tooth)
    (b"hteet", 5, true, true, &["tooth"]),
    // news (news)
    (b"swen", 4, true, true, &["news"]),
    // series (series)
    (b"seires", 6, true, true, &["series"]),
    // babies (baby)
    (b"sei", 3, false, true, &["y"]),
    // accesses (access), addresses (address), kisses (kiss)
    (b"sess", 4, true, false, &["ss"]),
    // statuses (status)
    (b"sesutats", 8, true, true, &["status"]),
    // article (articles), ancle (ancles)
    (b"sel", 3, true, true, &["le"]),
    // analyses (analysis), ellipses (ellipsis), fungi (fungus),
    // neuroses (neurosis), theses (thesis), emphases (emphasis),
    // oases (oasis), crises (crisis), houses (house), bases (base),
    // atlases (atlas)
    (b"ses", 3, true, true, &["s", "se", "sis"]),
    // objectives (objective), alternative (alternatives)
    (b"sevit", 5, true, true, &["tive"]),
    // drives (drive)
    (b"sevird", 6, false, true, &["drive"]),
    // knives (knife), jackknives (jackknife)
    (b"sevink", 6, true, true, &["knife"]),
    // wives (wife), midwives (midwife)
    (b"seviw", 5, true, true, &["wife"]),
    // lives (life)
    (b"sevil", 5, false, true, &["life"]),
    // archives (archive), explosives (explosive), hives (hive), olives (olive)
    (b"sevi", 4, true, true, &["ive"]),
    // moves (move)
    (b"sevom", 5, true, true, &["move"]),
    // hooves (hoof), dwarves (dwarf), elves (elf), leaves (leaf), caves (cave), staves (staff)
    (b"sev", 3, true, true, &["f", "ve", "ff"]),
    // axes (axis), axes (ax), axes (axe)
    (b"sexa", 4, false, false, &["ax", "axe", "axis"]),
    // indexes (index), matrixes (matrix)
    (b"sex", 3, true, false, &["x"]),
    // quizzes (quiz)
    (b"sezz", 4, true, false, &["z"]),
    // bureaus (bureau)
    (b"suae", 4, false, true, &["eau"]),
    // fees (fee), trees (tree), employees (employee)
    (b"see", 3, true, true, &["ee"]),
    // edges (edge)
    (b"segd", 4, true, true, &["dge"]),
    // outages (outage) - specific fix to avoid 'outag'
    (b"segatuo", 7, true, true, &["outage"]),
    // traces (trace), faces (face), places (place), pieces (piece)
    (b"sec", 3, true, true, &["ce"]),
    // roses (rose), garages (garage), cassettes (cassette),
    // waltzes (waltz), heroes (hero), bushes (bush), arches (arch),
    // shoes (shoe)
    (b"se", 2, true, true, &["", "e"]),
    // status (status)
    (b"sutats", 6, true, true, &["status"]),
    // tags (tag)
    (b"s", 1, true, true, &[""]),
    // chateaux (chateau)
    (b"xuae", 4, false, true, &["eau"]),
    // people (person)
    (b"elpoep", 6, true, true, &["person"]),
];

/// Map English singular to plural suffixes.
///
/// See <http://english-zone.com/spelling/plurals.html>
const SINGULAR_MAP: &[Map] = &[
    // passerby (passersby)
    (b"ybressap", 8, true, true, &["passersby"]),
    // insigne (insignia, insignias)
    (b"engisni", 7, true, true, &["insignia", "insignias"]),
    // nodes (node)
    (b"edon", 4, true, true, &["nodes"]),
    // axes (axis)
    (b"sixa", 4, false, false, &["axes"]),
    // criterion (criteria)
    (b"airetirc", 8, false, false, &["criterion"]),
    // nebulae (nebula)
    (b"aluben", 6, false, false, &["nebulae"]),
    // children (child)
    (b"dlihc", 5, true, true, &["children"]),
    // prices (price)
    (b"eci", 3, false, true, &["ices"]),
    // services (service)
    (b"ecivres", 7, true, true, &["services"]),
    // lives (life), wives (wife)
    (b"efi", 3, false, true, &["ives"]),
    // selfies (selfie)
    (b"eifles", 6, true, true, &["selfies"]),
    // movies (movie)
    (b"eivom", 5, true, true, &["movies"]),
    // lice (louse)
    (b"esuol", 5, false, true, &["lice"]),
    // mice (mouse)
    (b"esuom", 5, false, true, &["mice"]),
    // geese (goose)
    (b"esoo", 4, false, true, &["eese"]),
    // houses (house), bases (base)
    (b"es", 2, true, true, &["ses"]),
    // geese (goose)
    (b"esoog", 5, true, true, &["geese"]),
    // caves (cave)
    (b"ev", 2, true, true, &["ves"]),
    // drives (drive)
    (b"evird", 5, false, true, &["drives"]),
    // objectives (objective), alternative (alternatives)
    (b"evit", 4, true, true, &["tives"]),
    // moves (move)
    (b"evom", 4, true, true, &["moves"]),
    // staves (staff)
    (b"ffats", 5, true, true, &["staves"]),
    // hooves (hoof), dwarves (dwarf), elves (elf), leaves (leaf)
    (b"ff", 2, true, true, &["ffs"]),
    // hooves (hoof), dwarves (dwarf), elves (elf), leaves (leaf)
    (b"f", 1, true, true, &["fs", "ves"]),
    // arches (arch)
    (b"hc", 2, true, true, &["ches"]),
    // bushes (bush)
    (b"hs", 2, true, true, &["shes"]),
    // teeth (tooth)
    (b"htoot", 5, true, true, &["teeth"]),
    // albums (album)
    (b"mubla", 5, true, true, &["albums"]),
    // quorums (quorum)
    (b"murouq", 6, true, true, &["quora", "quorums"]),
    // bacteria (bacterium), curricula (curriculum), media (medium), memoranda (memorandum), phenomena (phenomenon), strata (stratum)
    (b"mu", 2, true, true, &["a"]),
    // men (man), women (woman)
    (b"nam", 3, true, true, &["men"]),
    // people (person)
    (b"nosrep", 6, true, true, &["persons", "people"]),
    // criteria (criterion)
    (b"noiretirc", 9, true, true, &["criteria"]),
    // phenomena (phenomenon)
    (b"nonemonehp", 10, true, true, &["phenomena"]),
    // echoes (echo)
    (b"ohce", 4, true, true, &["echoes"]),
    // heroes (hero)
    (b"oreh", 4, true, true, &["heroes"]),
    // atlases (atlas)
    (b"salta", 5, true, true, &["atlases"]),
    // aliases (alias)
    (b"saila", 5, true, true, &["aliases"]),
    // irises (iris)
    (b"siri", 4, true, true, &["irises"]),
    // analyses (analysis), ellipses (ellipsis), neuroses (neurosis)
    // theses (thesis), emphases (emphasis), oases (oasis),
    // crises (crisis)
    (b"sis", 3, true, true, &["ses"]),
    // accesses (access), addresses (address), kisses (kiss)
    (b"ss", 2, true, false, &["sses"]),
    // syllabi (syllabus)
    (b"suballys", 8, true, true, &["syllabi"]),
    // buses (bus)
    (b"sub", 3, true, true, &["buses"]),
    // circuses (circus)
    (b"suc", 3, true, true, &["cuses"]),
    // hippocampi (hippocampus)
    (b"supmacoppih", 11, false, false, &["hippocampi"]),
    // campuses (campus)
    (b"sup", 3, true, true, &["puses"]),
    // status (status)
    (b"sutats", 6, true, true, &["status", "statuses"]),
    // conspectuses (conspectus), prospectuses (prospectus)
    (b"sutcep", 6, true, true, &["pectuses"]),
    // nexuses (nexus)
    (b"suxen", 5, false, false, &["nexuses"]),
    // fungi (fungus), alumni (alumnus), syllabi (syllabus), radii (radius)
    (b"su", 2, true, true, &["i"]),
    // news (news)
    (b"swen", 4, true, true, &["news"]),
    // feet (foot)
    (b"toof", 4, true, true, &["feet"]),
    // chateaux (chateau), bureaus (bureau)
    (b"uae", 3, false, true, &["eaus", "eaux"]),
    // oxen (ox)
    (b"xo", 2, false, false, &["oxen"]),
    // hoaxes (hoax)
    (b"xaoh", 4, true, false, &["hoaxes"]),
    // indices (index)
    (b"xedni", 5, false, true, &["indicies", "indexes"]),
    // fax (faxes, faxxes)
    (b"xaf", 3, true, true, &["faxes", "faxxes"]),
    // boxes (box)
    (b"xo", 2, false, true, &["oxes"]),
    // indexes (index), matrixes (matrix), appendices (appendix)
    (b"x", 1, true, false, &["ces", "xes"]),
    // babies (baby)
    (b"y", 1, false, true, &["ies"]),
    // quizzes (quiz)
    (b"ziuq", 4, true, false, &["quizzes"]),
    // waltzes (waltz)
    (b"z", 1, true, true, &["zes"]),
];

/// A list of words which should not be inflected, reversed.
const UNINFLECTED: &[&[u8]] = &[
    b"",
    // data
    b"atad",
    // deer
    b"reed",
    // equipment
    b"tnempiuqe",
    // feedback
    b"kcabdeef",
    // fish
    b"hsif",
    // health
    b"htlaeh",
    // history
    b"yrotsih",
    // info
    b"ofni",
    // information
    b"noitamrofni",
    // money
    b"yenom",
    // moose
    b"esoom",
    // series
    b"seires",
    // sheep
    b"peehs",
    // species
    b"seiceps",
    // traffic
    b"ciffart",
    // aircraft
    b"tfarcria",
    // hardware
    b"erawdrah",
];

/// `Symfony\Component\String\Inflector\EnglishInflector`.
#[derive(Debug, Clone, Copy, Default)]
pub struct EnglishInflector;

/// `ucfirst()`: the first byte in ASCII upper case.
fn ucfirst(s: &str) -> Vec<u8> {
    let mut out = s.as_bytes().to_vec();
    if let Some(first) = out.first_mut() {
        first.make_ascii_uppercase();
    }
    out
}

/// The loop `singularize()` and `pluralize()` share: the first table
/// entry whose suffix ends the word (and may follow the letter before
/// it) gives the forms; `None` when no entry does.
fn inflect(word: &[u8], map: &[Map]) -> Option<Vec<Vec<u8>>> {
    let rev: Vec<u8> = word.iter().rev().copied().collect();
    let lower_rev = rev.to_ascii_lowercase();
    let length = lower_rev.len();
    for &(suffix, suffix_length, after_vowel, after_consonant, new_suffix) in map {
        let mut j = 0;
        // Compare characters in the table and of the suffix of the given
        // word one by one
        while suffix.get(j).is_some() && suffix.get(j) == lower_rev.get(j) {
            j += 1;
            // Successfully compared the last character
            if j == suffix_length {
                // Is there any character preceding the suffix in the word?
                if j < length {
                    let next_is_vowel = b"aeiou".contains(&lower_rev[j]);
                    if !after_vowel && next_is_vowel {
                        // suffix may not succeed a vowel but next char is one
                        break;
                    }
                    if !after_consonant && !next_is_vowel {
                        // suffix may not succeed a consonant but next char is one
                        break;
                    }
                }
                let base = &word[..length - suffix_length];
                // Keep the case of the suffix's first letter
                let first_upper = rev[j - 1].is_ascii_uppercase();
                return Some(
                    new_suffix
                        .iter()
                        .map(|entry| {
                            let entry = if first_upper {
                                ucfirst(entry)
                            } else {
                                entry.as_bytes().to_vec()
                            };
                            [base, &entry].concat()
                        })
                        .collect(),
                );
            }
            // Suffix is longer than word
            if j == length {
                break;
            }
        }
    }
    None
}

fn is_uninflected(word: &[u8]) -> bool {
    let lower_rev: Vec<u8> = word.iter().rev().map(u8::to_ascii_lowercase).collect();
    UNINFLECTED.contains(&lower_rev.as_slice())
}

impl EnglishInflector {
    pub fn new() -> Self {
        Self
    }

    /// `singularize()`, which cannot fail.
    pub fn singular_forms(&self, plural: &[u8]) -> Vec<Vec<u8>> {
        // Check if the word is one which is not inflected, return early if so
        if is_uninflected(plural) {
            return vec![plural.to_vec()];
        }
        // Assume that plural and singular is identical
        inflect(plural, PLURAL_MAP).unwrap_or_else(|| vec![plural.to_vec()])
    }

    /// `pluralize()`, which cannot fail.
    pub fn plural_forms(&self, singular: &[u8]) -> Vec<Vec<u8>> {
        // Check if the word is one which is not inflected, return early if so
        if is_uninflected(singular) {
            return vec![singular.to_vec()];
        }
        // Assume that plural is singular with a trailing `s`
        inflect(singular, SINGULAR_MAP).unwrap_or_else(|| vec![[singular, b"s"].concat()])
    }
}

impl Inflector for EnglishInflector {
    fn singularize(&self, plural: &[u8]) -> Result<Vec<Vec<u8>>> {
        Ok(self.singular_forms(plural))
    }

    fn pluralize(&self, singular: &[u8]) -> Result<Vec<Vec<u8>>> {
        Ok(self.plural_forms(singular))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Upstream's second entry is the first one's length.
    #[test]
    fn suffix_lengths_match_suffixes() {
        for (suffix, length, ..) in PLURAL_MAP.iter().chain(SINGULAR_MAP) {
            assert_eq!(suffix.len(), *length, "{}", String::from_utf8_lossy(suffix));
        }
    }
}
