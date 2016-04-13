use core::reader::SegmentReader;
use core::index::Index;
use core::index::Segment;
use core::schema::DocId;
use core::schema::Document;
use core::collector::Collector;
use std::io;
use core::timer::TimerTree;
use core::schema::Term;

#[derive(Debug)]
pub struct Searcher {
    segments: Vec<SegmentReader>,
}

/// A segment local id identifies a segment.
/// It only makes sense for a given searcher.
pub type SegmentLocalId = u32;

#[derive(Debug)]
pub struct DocAddress(pub SegmentLocalId, pub DocId);

impl Searcher {

    pub fn doc(&self, doc_address: &DocAddress) -> io::Result<Document> {
        // TODO err
        let DocAddress(ref segment_local_id, ref doc_id) = *doc_address;
        let segment_reader = &self.segments[*segment_local_id as usize];
        segment_reader.doc(doc_id)
    }

    fn add_segment(&mut self, segment: Segment) -> io::Result<()> {
        let segment_reader = try!(SegmentReader::open(segment.clone()));
        self.segments.push(segment_reader);
        Ok(())
    }

    fn new() -> Searcher {
        Searcher {
            segments: Vec::new(),
        }
    }

    pub fn for_index(index: Index) -> io::Result<Searcher> {
        let mut searcher = Searcher::new();
        for segment in index.segments().into_iter() {
            try!(searcher.add_segment(segment));
        }
        Ok(searcher)
    }

    pub fn search<C: Collector>(&self, terms: &Vec<Term>, collector: &mut C) -> io::Result<TimerTree> {
        let mut timer_tree = TimerTree::new();
        {
            let mut search_timer = timer_tree.open("search");
            for (segment_ord, segment) in self.segments.iter().enumerate() {
                let mut segment_search_timer = search_timer.open("segment_search");
                {
                    let set_segment_timer = segment_search_timer.open("set_segment");
                    try!(collector.set_segment(segment_ord as SegmentLocalId, &segment));
                }
                let postings = segment.search(terms, segment_search_timer.open("get_postings"));
                {
                    let collection_timer = segment_search_timer.open("collection");
                    for doc_id in postings {
                        collector.collect(doc_id);
                    }
                }
            }
        }
        Ok(timer_tree)
    }

}
