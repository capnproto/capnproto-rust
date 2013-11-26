/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

extern mod extra;

use std;
use std::rand::*;
use capnp;
use common::*;
use catrank_capnp::*;

pub type RequestBuilder = SearchResultList::Builder;
pub type ResponseBuilder = SearchResultList::Builder;
pub type Expectation = int;

pub fn newRequestReader<'a>(sr : capnp::layout::StructReader<'a>) -> SearchResultList::Reader<'a> {
    SearchResultList::Reader::new(sr)
}

pub fn newResponseReader<'a>(sr : capnp::layout::StructReader<'a>) -> SearchResultList::Reader<'a> {
    SearchResultList::Reader::new(sr)
}

pub struct ScoredResult<'self> {
    score : f64,
    result : SearchResult::Reader<'self>
}

static URL_PREFIX : &'static str = "http://example.com";

pub fn setupRequest(rng : &mut FastRand, request : SearchResultList::Builder) -> int {
    let count = rng.nextLessThan(1000) as uint;
    let mut goodCount : int = 0;

    let list = request.initResults(count);

    for i in range(0, count) {
        let result = list[i];
        result.setScore(1000.0 - i as f64);
        let urlSize = rng.nextLessThan(100) as uint;

        // TODO: modify string field in place with Text::Builder?
        let mut url = ~"http://example.com/";

        for _ in range(0, urlSize) {
            url.push_char(std::char::from_u32(97 + rng.nextLessThan(26)).unwrap());
        }

        result.setUrl(url);

        let isCat = rng.nextLessThan(8) == 0;
        let isDog = rng.nextLessThan(8) == 0;
        if (isCat && !isDog) {
            goodCount += 1;
        }

        let mut snippet = ~" ";

        let prefix = rng.nextLessThan(20) as uint;
        for _ in range(0, prefix) {
            snippet.push_str(WORDS[rng.nextLessThan(WORDS.len() as u32)]);
        }
        if (isCat) { snippet.push_str("cat ") }
        if (isDog) { snippet.push_str("dog ") }

        let suffix = rng.nextLessThan(20) as uint;
        for _ in range(0, suffix) {
            snippet.push_str(WORDS[rng.nextLessThan(WORDS.len() as u32)]);
        }

        result.setSnippet(snippet);
    }

    goodCount
}

pub fn handleRequest(request : SearchResultList::Reader,
                     response : SearchResultList::Builder) {
    let mut scoredResults : ~[ScoredResult] = ~[];

    let results = request.getResults();
    for i in range(0, results.size()) {
        let result = results[i];
        let mut score = result.getScore();
        if (result.getSnippet().contains(" cat ")) {
            score *= 10000.0;
        }
        if (result.getSnippet().contains(" dog ")) {
            score /= 10000.0;
        }
        scoredResults.push(ScoredResult {score : score, result : result});
    }

    extra::sort::quick_sort(scoredResults, |v1, v2| {v1.score <= v2.score });

    let list = response.initResults(scoredResults.len());
    for i in range(0, list.size()) {
        let item = list[i];
        let result = scoredResults[i];
        item.setScore(result.score);
        item.setUrl(result.result.getUrl());
        item.setSnippet(result.result.getSnippet());
    }
}

pub fn checkResponse(response : SearchResultList::Reader, expectedGoodCount : int) -> bool {
    let mut goodCount : int = 0;
    let results = response.getResults();
    for i in range(0, results.size()) {
        let result = results[i];
        if (result.getScore() > 1001.0) {
            goodCount += 1;
        } else {
            break;
        }
    }
    return goodCount == expectedGoodCount;
}
