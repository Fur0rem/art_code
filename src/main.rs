#![allow(clippy::needless_return)]
#![allow(clippy::iter_nth_zero)]

use tree_sitter::Parser;
use image::{GenericImageView};
use itertools::Itertools;

use std::fs::{read_to_string,File};
use std::io::Write;
use std::process::*;
use std::env;


fn get_str_ascii(intent : usize) -> &'static str{
      const ASCII : [&str; 9] = [" ",".","~","=","+","0","8","@","#"];
      return ASCII[(intent/32)];
}

fn image_to_ascii(dir: &str, scale: usize) -> String{
      
      let mut str_art = String::new();
      let img = match image::open(dir) {
            Ok(image) => image,
            Err(_) => panic!("Couldn't open art image : {dir}")
      };
      
      let (width,height) = img.dimensions();
      for y in (0..height).step_by(scale*2){
            for x in (0..width).step_by(scale){
                  
                  let pix = img.get_pixel(x,y);
                  
                  let intent = match pix[3]{
                        0 => 0,
                        _ => 32 + (pix[0]/3 + pix[1]/3 + pix[2]/3) as usize
                  };
                  
                  str_art.push_str(get_str_ascii(intent));
            }
            str_art.push('\n');
      }
      return str_art;
}


const BLOCK_COMMENT : u16 = 138;
const LINE_COMMENT : u16 = 130;
const STRING_LITERAL : u16 = 289;
const TOKEN_TREE : u16 = 150;
const IDENTIFIER : u16 = 1;
const LIFETIME : u16 = 201;
const TYPE_PARAMETERS : u16 = 181;
const GENERIC_TYPE : u16 = 208;
const REFERENCE_TYPE : u16 = 213;
const INNER_ATTRIBUTE_ITEM : u16 = 153;


fn ast_2_string(code : &[u8], root : &tree_sitter::Node) -> String{
      
      match root.kind_id(){
            
            LINE_COMMENT | BLOCK_COMMENT => String::new(),
            
            STRING_LITERAL => return root.utf8_text(code).unwrap().to_string(),
            
            INNER_ATTRIBUTE_ITEM => return root.utf8_text(code).unwrap().to_string() + "\n",
            
            TYPE_PARAMETERS | GENERIC_TYPE | LIFETIME | REFERENCE_TYPE => return root.utf8_text(code).unwrap().to_string() + " ",
            
            TOKEN_TREE => {   
                  
                  let mut str_tree = String::from(root.child(0).unwrap().utf8_text(code).unwrap());
                  let nb_children = root.child_count();
                  
                  for child in 1..nb_children {
                        
                        let byte_ref = root.child(child).unwrap().start_byte();
                        if (code[byte_ref-1] as char=='&') || (code[byte_ref] as char=='&'){
                              str_tree.push('&');
                        }
                        
                        match root.child(child).unwrap().utf8_text(code).unwrap() {
                              
                              ")" | "(" | "[" | "]" => break,
                              
                              _ => {
                                    str_tree.push_str(&ast_2_string(code, &root.child(child).unwrap()));
                                    if child<nb_children-2 && 
                                    (root.child(child).unwrap().kind_id()!=IDENTIFIER || 
                                    root.child(child+1).unwrap().kind_id()!=TOKEN_TREE ) {
                                          str_tree.push_str(",\n");
                                    }
                              }
                              
                        }
                  }
                  
                  str_tree.push('\n');
                  str_tree.push_str(root.child(nb_children-1).unwrap().utf8_text(code).unwrap());
                  str_tree.push('\n');
                  
                  let str_tree_vec = str_tree.chars().collect();
                  let code_vec = root.utf8_text(code).unwrap().chars().collect::<Vec<char>>();
                  let mut corrected_str_tree_vec = correct_exceptions(&code_vec, &str_tree_vec);
                  rectify_text(&code_vec, &mut corrected_str_tree_vec);
                  return corrected_str_tree_vec.into_iter().collect();
                  
            }
            
            
            _ => {
                  let mut str_tree = String::new();
                  let nb_children = root.child_count();
                  
                  if nb_children==0{
                        str_tree.push_str(root.utf8_text(code).unwrap());
                        str_tree.push('\n');
                  }
                  
                  else {
                        for child in 0..nb_children{
                              str_tree.push_str(&ast_2_string(code, &root.child(child).unwrap()));
                        }
                  }
                  
                  return str_tree;
            }
      }
}

const EXCEPTIONS : [&str;3] = ["<=",">=","=="];
fn correct_exceptions(code : &[char], bad_code : &Vec<char>) -> Vec<char>{
      
      let bad_code_tuples : Vec<_> = bad_code.iter().tuple_windows::<_>().collect();
      let code_tuples : Vec<_> = code.iter().tuple_windows::<_>().collect();
      
      let mut nouv_code : Vec<(&char,&char)> = Vec::new();
      let mut i = 0;
      let mut j = 0;
      
      while i<code_tuples.len()-1 {
            if code_tuples[i]==bad_code_tuples[j]{
                  nouv_code.push(code_tuples[i]);
                  i+=1;j+=1;
            }
            else if !EXCEPTIONS.contains(&format!("{}{}",code_tuples[i].0,code_tuples[i].1).as_str()){
                  nouv_code.push(bad_code_tuples[j]);
                  i+=1;j+=1;
                  
            }
            else{
                  nouv_code[i-1]=(&'\n',code_tuples[i-1].1);
                  nouv_code.push(code_tuples[i]);
                  nouv_code.push(code_tuples[i+1]);
                  nouv_code[i+1].1=&'\n';
                  i+=1;j+=1;
            }
      }
      
      let mut nouv_string = String::new();
      nouv_string.push('\n');
      nouv_string.push(*code_tuples[0].0);
      nouv_string.push('\n');
      for couple in nouv_code{
            nouv_string.push(*couple.1);
      }
      nouv_string.push(*code_tuples[i].1);
      
      return nouv_string.chars().collect();
      
}


const ARRAY_WHITESPACES : [char ; 3] = [' ','\n','\t'];

fn rectify_text(correct_text : &Vec<char>, extracted_text : &mut Vec<char>){
      
      correct_exceptions(correct_text, extracted_text);
      
      let len1 = correct_text.len();
      let len2 = extracted_text.len();
      
      let mut i1 = 0;
      let mut i2 = 0;
      
      while i1 < len1 && i2 < len2{
            
            while i1<len1 && ARRAY_WHITESPACES.contains(&correct_text[i1]){ 
                  i1 += 1;
            }
            while i2<len2 && ARRAY_WHITESPACES.contains(&extracted_text[i2]){ 
                  i2 += 1;
            }
            
            if i2<len2 && i1<len1{
                  extracted_text[i2] = correct_text[i1];
                  i1 += 1;
                  i2 += 1;
            }
            
      }
      
      while i1<len1 {
            while ARRAY_WHITESPACES.contains(&correct_text[i1]) { 
                  i1 += 1;
            }
            extracted_text.push(correct_text[i1]);
            i1+=1;
      }
      
}


const COMMENT_CHARS : [char;2] = ['/','*'];

fn chars_to_fill_vec(chars : &[char], separator : char, vec_to_fill : &mut Vec<String>){
      
      vec_to_fill.clear();
      vec_to_fill.push(String::new());
      
      let mut c_line = 0;
      
      for &chr in chars.iter(){
            if chr == separator{
                  c_line += 1;
                  vec_to_fill.push(String::new());
            }
            else{
                  vec_to_fill[c_line].push(chr);
            }
      }
      
}

fn get_last_nwhitespace_index(words : &Vec<String>) -> usize{
      
      let mut ilast_nonwhite_word = words.len() - 1;
      while words[ilast_nonwhite_word].is_empty() && ilast_nonwhite_word > 0{
            ilast_nonwhite_word -= 1;
      }
      
      return ilast_nonwhite_word;
}


fn code_to_art(code : Vec<char>, ascii_art : String) -> String{
      
      let lines_art = ascii_art.lines().collect::<Vec<&str>>();
      
      let mut vec_code_wrds = Vec::with_capacity(code.len()/2); 
      chars_to_fill_vec(&code, '\n', &mut vec_code_wrds);
      
      let mut string_final = String::new();
      let mut nb_code_wrds_put : usize = 0;
      let mut final_cmt_status : usize = 0;
      let mut final_cmt = String::from("");
      
      let mut vec_ascii_line : Vec<String> = Vec::with_capacity(ascii_art.lines().nth(0).unwrap().len());
      
      let mut nb_wrds_cur_line = vec_code_wrds.len() - 1;
      let mut string_cur_line=String::new();
      
      for line in lines_art.iter().map(|l| l.trim_end()){
            
            if nb_code_wrds_put!=nb_wrds_cur_line{
                  
                  chars_to_fill_vec(&line.chars().collect::<Vec<char>>(), ' ', &mut vec_ascii_line);
                  nb_wrds_cur_line = vec_code_wrds.len() - 1;
                  
                  string_cur_line.clear();
                  let i_last_nwhite_wrd = get_last_nwhitespace_index(&vec_ascii_line);
                  
                  for (i_vec_ascii,ascii_wrd) in vec_ascii_line.clone().into_iter().enumerate(){
                        
                        
                        let binding = ascii_wrd.clone();
                        let ascii_bytes = binding.as_bytes();
                        
                        //Si il y a du ASCII art qu'on peut potentiellement remplacer
                        if ascii_wrd != *"" {
                              
                              let mut ascii_len = ascii_wrd.len();
                              let mut code_wrd = &vec_code_wrds[nb_code_wrds_put];
                              let mut cur_wrd_len = code_wrd.len();
                              
                              //On case le plus de bouts de code qu'on peut
                              while (cur_wrd_len + 1 < ascii_len) && (nb_code_wrds_put < nb_wrds_cur_line) {
                                    string_cur_line.push_str(code_wrd);
                                    ascii_len -= cur_wrd_len + 1;
                                    string_cur_line.push(' ');
                                    nb_code_wrds_put += 1;
                                    code_wrd = &vec_code_wrds[nb_code_wrds_put];
                                    cur_wrd_len = code_wrd.len();
                              }
                              
                              //Bouts de phrases au milieu dont on a pas réussi à casser du code pour
                              if ascii_len > 4{
                                    string_cur_line.push_str("/*");
                                    ascii_len -= 2;
                                    while ascii_len>2{
                                          string_cur_line.push(ascii_bytes[ascii_len] as char);
                                          ascii_len -= 1;
                                    }
                                    string_cur_line.push_str("*/");
                              }
                              
                              //Bouts de phrases à la fin dont on a pas réussi à casser du code pour
                              else if ascii_len >= 2 && i_vec_ascii==i_last_nwhite_wrd{
                                    string_cur_line.push_str("//");
                                    ascii_len -= 2;
                                    while ascii_len>0{
                                          string_cur_line.push(ascii_bytes[ascii_len] as char);
                                          ascii_len -= 1;
                                    }
                              }
                              
                        }
                        
                        string_cur_line.push(' '); //S'il n'y rien à cet endroit dans le ASCII art
                  }
                  
                  string_cur_line = string_cur_line.trim_end().to_string();
                  string_final.push_str(&string_cur_line);
                  string_final.push('\n');
            }
            
            else{
                  
                  let mut i=0;
                  while i<line.len() {
                        
                        let character = line.as_bytes()[i] as char;
                        
                        if ['\t',' '].contains(&character){
                              final_cmt.push(character);
                        }
                        
                        else{
                              
                              match final_cmt_status{
                                    0 => { 
                                          final_cmt.push_str("/*");
                                          final_cmt_status += 1;
                                          i+=1;
                                    }
                                    1 => {  
                                          final_cmt.push(COMMENT_CHARS[final_cmt_status]);
                                          final_cmt_status += 1;
                                    }
                                    _ => final_cmt.push(character)
                              }
                              
                        }
                        i+=1;
                  }
                  
                  final_cmt.push('\n');
                  
            }
      }
      
      string_final.push_str(&final_cmt);
      string_final = string_final.trim_end().to_string();
      
      //Deuxieme balise pour le commentaire
      if final_cmt_status>=1{
            string_final.pop();
            string_final.pop();
            if final_cmt_status > 1 {
                  string_final.push_str("*/");
            }
      }
      
      return string_final;
}

fn main() -> ExitCode{
      
      let args: Vec<String> = env::args().collect();
      assert!(args.len() == 4, "{}", format!("Usage: {} <Path_to_code> <Path_to_art> <Path_to_result>",args[0]));
      
      let code_whitespace = read_to_string(&args[1]).unwrap().chars().collect::<String>();
      let code = code_whitespace.trim_end();
      
      
      let mut parser = Parser::new();
      parser.set_language(tree_sitter_rust::language()).expect("Error loading Rust grammar");
      let parsed = parser.parse(code, None).unwrap();
      let source_as_vec = code.as_bytes();
      
      let vec_code = ast_2_string(source_as_vec, &parsed.root_node()).chars().collect::<Vec<char>>();
      let ascii_art = image_to_ascii(&args[2],1);
      let art_code_string = code_to_art(vec_code,ascii_art);
      
      let mut dest_file = File::create(&args[3]).expect("Couldn't create destination file");
      dest_file.write_all(art_code_string.as_bytes()).expect("Couldn't write to destination file");
      
      exit(0);
      
}