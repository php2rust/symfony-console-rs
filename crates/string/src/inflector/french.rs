//! Port of Symfony\Component\String\Inflector\FrenchInflector.php
//!
//! Symfony source copyright (c) Fabien Potencier; distributed under MIT.
//!
//! Only nouns are inflected; not adjectives nor composed words like
//! "soixante-dix". The patterns are upstream's, run by php-pcre without
//! `/u`: `/i` folds ASCII letters only, so "É" and "é" differ.
//!
//! Upstream: symfony/string Inflector/FrenchInflector.php

use std::sync::LazyLock;

use super::{Inflector, Rules, apply, compile};
use crate::{exception::Result, preg::Regex};

/// The rules for pluralize, in order.
///
/// See <https://la-conjugaison.nouvelobs.com/regles/grammaire/le-pluriel-des-noms-121.php>
const PLURALIZE_REGEXP: &[(&str, &str)] = &[
    // Words finishing with "s", "x" or "z" are invariables
    // Les mots finissant par "s", "x" ou "z" sont invariables
    ("/(s|x|z)$/i", "\\1"),
    // Words finishing with "eau" are pluralized with a "x"
    // Les mots finissant par "eau" prennent tous un "x" au pluriel
    ("/(eau)$/i", "\\1x"),
    // Words finishing with "au" are pluralized with a "x" excepted "landau"
    // Les mots finissant par "au" prennent un "x" au pluriel sauf "landau"
    ("/^(landau)$/i", "\\1s"),
    ("/(au)$/i", "\\1x"),
    // Words finishing with "eu" are pluralized with a "x" excepted "pneu", "bleu", "émeu"
    // Les mots finissant en "eu" prennent un "x" au pluriel sauf "pneu", "bleu", "émeu"
    ("/^(pneu|bleu|émeu)$/i", "\\1s"),
    ("/(eu)$/i", "\\1x"),
    // Words finishing with "al" are pluralized with a "aux" excepted
    // Les mots finissant en "al" se terminent en "aux" sauf
    (
        "/^(bal|carnaval|caracal|chacal|choral|corral|étal|festival|récital|val)$/i",
        "\\1s",
    ),
    ("/al$/i", "\\1aux"),
    // Aspirail, bail, corail, émail, fermail, soupirail, travail, vantail et vitrail font leur pluriel en -aux
    (
        "/^(aspir|b|cor|ém|ferm|soupir|trav|vant|vitr)ail$/i",
        "\\1aux",
    ),
    // Bijou, caillou, chou, genou, hibou, joujou et pou qui prennent un x au pluriel
    ("/^(bij|caill|ch|gen|hib|jouj|p)ou$/i", "\\1oux"),
    // Invariable words
    ("/^(cinquante|soixante|mille)$/i", "\\1"),
    // French titles
    ("/^(mon|ma)(sieur|dame|demoiselle|seigneur)$/", "mes\\2s"),
    ("/^(Mon|Ma)(sieur|dame|demoiselle|seigneur)$/", "Mes\\2s"),
];

/// The rules for singularize, in order.
const SINGULARIZE_REGEXP: &[(&str, &str)] = &[
    // Aspirail, bail, corail, émail, fermail, soupirail, travail, vantail et vitrail font leur pluriel en -aux
    (
        "/((aspir|b|cor|ém|ferm|soupir|trav|vant|vitr))aux$/i",
        "\\1ail",
    ),
    // Words finishing with "eau" are pluralized with a "x"
    // Les mots finissant par "eau" prennent tous un "x" au pluriel
    ("/(eau)x$/i", "\\1"),
    // Words finishing with "al" are pluralized with a "aux" expected
    // Les mots finissant en "al" se terminent en "aux" sauf
    (
        "/(amir|anim|arsen|boc|can|capit|capor|chev|crist|génér|hopit|hôpit|idé|journ|littor|loc|m|mét|minér|princip|radic|termin)aux$/i",
        "\\1al",
    ),
    // Words finishing with "au" are pluralized with a "x" excepted "landau"
    // Les mots finissant par "au" prennent un "x" au pluriel sauf "landau"
    ("/(au)x$/i", "\\1"),
    // Words finishing with "eu" are pluralized with a "x" excepted "pneu", "bleu", "émeu"
    // Les mots finissant en "eu" prennent un "x" au pluriel sauf "pneu", "bleu", "émeu"
    ("/(eu)x$/i", "\\1"),
    //  Words finishing with "ou" are pluralized with a "s" excepted bijou, caillou, chou, genou, hibou, joujou, pou
    // Les mots finissant par "ou" prennent un "s" sauf bijou, caillou, chou, genou, hibou, joujou, pou
    ("/(bij|caill|ch|gen|hib|jouj|p)oux$/i", "\\1ou"),
    // French titles
    ("/^mes(dame|demoiselle)s$/", "ma\\1"),
    ("/^Mes(dame|demoiselle)s$/", "Ma\\1"),
    ("/^mes(sieur|seigneur)s$/", "mon\\1"),
    ("/^Mes(sieur|seigneur)s$/", "Mon\\1"),
    // Default rule
    ("/s$/i", ""),
];

/// A list of words which should not be inflected. (Upstream's comment
/// says only singularize uses it; both methods check it.)
const UNINFLECTED: &str = "/^(abcès|accès|abus|albatros|anchois|anglais|autobus|bois|brebis|carquois|cas|chas|colis|concours|corps|cours|cyprès|décès|devis|discours|dos|embarras|engrais|entrelacs|excès|fils|fois|gâchis|gars|glas|héros|intrus|jars|jus|kermès|lacis|legs|lilas|marais|mars|matelas|mépris|mets|mois|mors|obus|os|palais|paradis|parcours|pardessus|pays|plusieurs|poids|pois|pouls|printemps|processus|progrès|puits|pus|rabais|radis|recors|recours|refus|relais|remords|remous|rictus|rhinocéros|repas|rubis|sans|sas|secours|sens|souris|succès|talus|tapis|tas|taudis|temps|tiers|univers|velours|verglas|vernis|virus)$/i";

static PLURALIZE: LazyLock<Rules> = LazyLock::new(|| compile(PLURALIZE_REGEXP));
static SINGULARIZE: LazyLock<Rules> = LazyLock::new(|| compile(SINGULARIZE_REGEXP));
static UNINFLECTED_RE: LazyLock<Result<Regex>> =
    LazyLock::new(|| Regex::new(UNINFLECTED.as_bytes(), "preg_match"));

/// `Symfony\Component\String\Inflector\FrenchInflector`.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrenchInflector;

impl FrenchInflector {
    pub fn new() -> Self {
        Self
    }

    fn is_inflected_word(word: &[u8]) -> Result<bool> {
        let regex = UNINFLECTED_RE.as_ref().map_err(Clone::clone)?;
        Ok(regex.first(word, 0)?.is_some())
    }
}

impl Inflector for FrenchInflector {
    fn singularize(&self, plural: &[u8]) -> Result<Vec<Vec<u8>>> {
        if Self::is_inflected_word(plural)? {
            return Ok(vec![plural.to_vec()]);
        }
        apply(&SINGULARIZE, plural, plural.to_vec())
    }

    fn pluralize(&self, singular: &[u8]) -> Result<Vec<Vec<u8>>> {
        if Self::is_inflected_word(singular)? {
            return Ok(vec![singular.to_vec()]);
        }
        apply(&PLURALIZE, singular, [singular, b"s"].concat())
    }
}
