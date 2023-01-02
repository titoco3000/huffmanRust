use std::env;
use std::io::BufReader;
use std::io::Read;
mod huffman;

//indica o tamanho de bytes de cada bloco
const BLOCK_SIZE: usize = 1;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Argumentos insuficientes");
        return Ok(());
    }

    //path_saida é usado, mas compilador não percebe...?
    #[allow(unused_assignments)]
    let mut path_saida = String::with_capacity(args[2].len() + 4);

    let mut entrada = Vec::new();

    //saida é usada, mas compilador não percebe...?
    #[allow(unused_assignments)]
    let mut saida = Vec::new();

    // Lê arquivo para um vetor
    let f = std::fs::File::open(&args[2])?;
    BufReader::new(f).read_to_end(&mut entrada)?;

    if args[1] == "compactar" {
        path_saida = if args.len() > 3 {
            args[3].clone()
        } else {
            //se não especificar saída, ela é o nome do arquivo com o final ".hfm" (.huffman)
            args[2].clone() + ".hfm"
        };
        saida = huffman::encode::<BLOCK_SIZE>(&mut entrada);
    } else if args[1] == "descompactar" {
        path_saida = if args.len() > 3 {
            args[3].clone()
        } else {
            //se não especificar saída
            //se é um .hfm
            if let Some(removido) = args[2].strip_suffix(".hfm") {
                removido.to_string()
            } else {
                args[2].clone()
            }
        };
        saida = huffman::decode::<BLOCK_SIZE>(&mut entrada);
    } else {
        println!(
            "Opção desconhecida: {}. As disponiveis são 'compactar' e 'descompactar'",
            args[1]
        );
        return Ok(());
    }

    //imprime o tamanho do arquivo inicial e final
    println!("{}B -> {}B", entrada.len(), saida.len());
    std::fs::write(path_saida, saida)?;
    Ok(())
}
