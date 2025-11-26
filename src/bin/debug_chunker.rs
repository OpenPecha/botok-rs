use botok_rs::Chunker;

fn main() {
    let input = "ཀ འདི་ ཤི དེ་ག རེད་དོ།";
    println!("Input: {}", input);
    
    let chunker = Chunker::new(input);
    let chunks = chunker.make_chunks();
    
    println!("\nChunks:");
    for (i, chunk) in chunks.iter().enumerate() {
        println!("  {}: {:?} - syl: {:?}, type: {:?}", 
            i, 
            &input[chunk.start..chunk.start + chunk.len],
            chunk.syl,
            chunk.chunk_type
        );
    }
    
    let text_chunks: Vec<_> = chunks.iter().filter(|c| c.chunk_type == botok_rs::ChunkType::Text).collect();
    println!("\nText chunks count: {}", text_chunks.len());
}

