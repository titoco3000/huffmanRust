use bitvec::prelude as bv;

/*
Cria uma arvore de huffman para a mensagem dada  e retorna um vetor que contém ela
mais a mensagem codificada. Cada bloco tem o tamanho de COUNT bytes
*/
pub fn encode<const COUNT: usize>(mensagem: &mut Vec<u8>) -> Vec<u8> {
    // adiciona o quantos faltam para a mensagem ter tamanho multiplo de COUNT
    for _ in 0..mensagem.len() % COUNT {
        mensagem.push(0);
    }

    //a capacidade inicial é a da tabela ASCII, porém pode passar disso se COUNT > 1
    let mut lista: Vec<Node<COUNT>> = Vec::with_capacity(255);

    let mut blocos_unicos = 0;
    let mut i = 0;
    //avança o buffer de block_size em block_size bytes, e vai contando frequencias
    while i < mensagem.len() {
        let r: [u8; COUNT] = mensagem[i..i + COUNT].try_into().expect("");
        if let Some(mut item) = find_pop(&mut lista, r) {
            item.freq += 1;
            lista.push(item);
        } else {
            lista.push(Node::new(Some(r)));
            blocos_unicos += 1;
        }
        i += COUNT;
    }

    //quantidade inicial de nodes
    let mut nodes_count = blocos_unicos;

    //monta a arvore de huffman
    while lista.len() > 1 {
        let first = pop_smallest(&mut lista).unwrap();
        let second = pop_smallest(&mut lista).unwrap();

        let mut novo_node = Node::new(None);
        nodes_count += 1;
        novo_node.freq = first.freq + second.freq;
        novo_node.esq = Some(Box::new(first));
        novo_node.dir = Some(Box::new(second));
        lista.push(novo_node);
    }

    //cria um vetor relacionando cada bloco com seu código prefixo
    let mut dicionario = Vec::<([u8; COUNT], bv::BitVec)>::with_capacity(blocos_unicos);
    let bits_msg = generate_codes(&lista[0], &mut dicionario, &mut bv::BitVec::new());

    //ordena esse vetor para poder buscar em O(LgN) depois
    dicionario.sort_by(|a, b| cmp_byte_arrays(&a.0, &b.0));

    //metadata (cabeçalho) possui 32 bits indicam a quantidade de blocos no corpo,
    //mais cada um dos blocos, mais 1 bit cada um dos nós da árvore
    let bits_metadata: usize = 32 + blocos_unicos * 8 * COUNT + nodes_count;

    //agora que tenho todas as correspondencias e tamanhos,
    //começa a montar o vetor de saída

    //cria o vetor de saída
    let mut coded = bv::BitVec::with_capacity(bits_msg + bits_metadata);

    //adiciona a árvore de huffman
    push_tree_into_bitvec(&mut lista[0], &mut coded);

    //adiciona o número de blocos no corpo
    push_num_into_bitvec((mensagem.len() / COUNT) as u32, 32, &mut coded);

    let mut bloco_construido = [0u8; COUNT];
    let mut bytes_bloco_construido = 0;

    for byte in mensagem {
        //vai montando blocos, cada um composto de COUNT bytes.
        bloco_construido[bytes_bloco_construido] = byte.clone();
        bytes_bloco_construido += 1;
        //o bloco está pronto
        if bytes_bloco_construido == COUNT {
            if let Some(codigo) = busca_binaria_dicionario(&dicionario, &bloco_construido) {
                //adiciona o código desse bloco
                coded.append(&mut codigo.clone());
            } else {
                //não deveria acontecer, mas se no futuro algo quebrar, ajuda a encontrar o erro
                panic!(
                    "Bloco não encontrado: {:?}\ndentre\n {:#?}",
                    bloco_construido, dicionario
                );
            }
            bloco_construido = [0u8; COUNT];
            bytes_bloco_construido = 0;
        }
    }
    //converte em bytes
    bitvec_to_bytevec(&coded)
}

//cria uma arvore a partir dos metadados, decodifica a mensagem e devolve
pub fn decode<const COUNT: usize>(mensagem: &mut Vec<u8>) -> Vec<u8> {
    let mut bitvec = bv::BitVec::with_capacity(mensagem.len());
    //preenche o vetor de bits
    for byte in mensagem {
        push_byte_array_into_bitvec(&[*byte], &mut bitvec);
    }

    //para ter referencia de qual bit está sendo lido atualmente
    let mut indice = 0;
    //monta a arvore
    let tree = get_tree_from_bitvec::<COUNT>(&bitvec, &mut indice);

    //obtém o tamanho da mensagem
    let blocos = get_u32_from_bitvec(&bitvec, &mut indice, 32) as usize;

    //vetor de bytes de saída
    let mut decoded = Vec::with_capacity(blocos * COUNT);

    //para cada um dos blocos
    for _ in 0..blocos {
        let mut galho = &tree;

        //enquanto não chegar em uma folha da árvore
        loop {
            //se direita
            if bitvec[indice] {
                galho = galho.dir.as_ref().unwrap();
            }
            //se esquerda
            else {
                galho = galho.esq.as_ref().unwrap();
            }
            indice += 1;
            //se chegou em uma folha
            if let Some(valor) = galho.valor {
                //adiciona cada um dos bytes do bloco
                for byte in valor {
                    decoded.push(byte);
                }
                break;
            }
        }
    }
    decoded
}

#[derive(Debug)]
//struct da arvore de Huffman. COUNT diz quantos bytes por node
struct Node<const COUNT: usize> {
    valor: Option<[u8; COUNT]>,
    freq: usize,
    esq: Option<Box<Node<COUNT>>>,
    dir: Option<Box<Node<COUNT>>>,
}
impl<const COUNT: usize> Node<COUNT> {
    fn new(valor: Option<[u8; COUNT]>) -> Node<COUNT> {
        let freq = if valor.is_some() { 1 } else { 0 };
        Node {
            valor,
            freq,
            esq: None,
            dir: None,
        }
    }
}

//encontra e retira o valor do vetor de nodes
fn find_pop<const COUNT: usize>(
    lista: &mut Vec<Node<COUNT>>,
    value: [u8; COUNT],
) -> Option<Node<COUNT>> {
    for i in 0..lista.len() {
        if let Some(v) = &(lista[i].valor) {
            if *v == value {
                //swap remove para evitar O(N) na remoção
                return Some(lista.swap_remove(i));
            }
        }
    }
    None
}

//encontra e retira o menor valor do vetor de nodes
fn pop_smallest<const COUNT: usize>(lista: &mut Vec<Node<COUNT>>) -> Option<Node<COUNT>> {
    if lista.is_empty() {
        return None;
    }
    let mut menor_freq = lista[0].freq;
    let mut menor_index = 0;

    for i in 1..lista.len() {
        if lista[i].freq < menor_freq {
            menor_freq = lista[i].freq;
            menor_index = i;
        }
    }
    Some(lista.swap_remove(menor_index))
}

//gera os codigos e conta quantos bits são necessários para codificar a mensagem usando eles.
//Basicamente percorre em ordem a árvore, somando tamanhos*frequencias
fn generate_codes<const COUNT: usize>(
    tree: &Node<COUNT>,
    lista: &mut Vec<([u8; COUNT], bv::BitVec)>,
    caminho: &mut bv::BitVec,
) -> usize {
    let mut bits_totais: usize = 0;
    //se tem filhos
    if let (Some(esq), Some(dir)) = (&tree.esq, &tree.dir) {
        //informa o caminho
        caminho.push(false);
        //gera da esquerda
        bits_totais += generate_codes(esq, lista, caminho);

        //informa o caminho
        *caminho.last_mut().unwrap() = true;
        //gera da direita
        bits_totais += generate_codes(dir, lista, caminho);

        //volta atras no caminho
        caminho.truncate(caminho.len() - 1);
    } else {
        //adiciona o caminho dessa folha
        lista.push((tree.valor.unwrap(), caminho.clone()));

        bits_totais += tree.freq * caminho.len();
    }
    bits_totais
}

//faz uma busca binaria no vetor que relaciona blocos e códigos
fn busca_binaria_dicionario<'a, const COUNT: usize>(
    dic: &'a Vec<([u8; COUNT], bv::BitVec)>,
    valor: &[u8; COUNT],
) -> Option<&'a bv::BitVec> {
    let mut esq = 0;
    let mut dir = dic.len();
    while esq < dir {
        let meio = (esq + dir) / 2;
        //meio < valor
        if cmp_byte_arrays(&dic[meio].0, valor) == std::cmp::Ordering::Less {
            esq = meio + 1;
        }
        //meio > valor
        else if cmp_byte_arrays(&dic[meio].0, valor) == std::cmp::Ordering::Greater {
            dir = meio;
        }
        //meio == valor
        else {
            return Some(&dic[meio].1);
        }
    }
    None
}

//compara dois arrays de bytes
fn cmp_byte_arrays(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    let mut byte_atual: usize = std::cmp::max(a.len(), b.len());
    while byte_atual > 0 {
        byte_atual -= 1;
        let val_a = if a.len() > byte_atual {
            a[byte_atual]
        } else {
            0
        };
        let val_b = if b.len() > byte_atual {
            b[byte_atual]
        } else {
            0
        };
        return val_a.cmp(&val_b);
    }
    std::cmp::Ordering::Equal
}

//adiciona um número com o tamanho especificado em um BitVec
fn push_num_into_bitvec(valor: u32, tamanho: u8, bitvec: &mut bv::BitVec) {
    for i in (32 - tamanho)..32 {
        bitvec.push((valor >> 31 - i) % 2 == 1);
    }
}

//adiciona os bytes em um BitVec
fn push_byte_array_into_bitvec<const COUNT: usize>(valor: &[u8; COUNT], bitvec: &mut bv::BitVec) {
    for byte in valor {
        for i in 0..8 {
            bitvec.push((byte >> 7 - i) % 2 == 1);
        }
    }
}

/*
adiciona recursivamente uma árvore de huffman em um BitVec.
Se for folha: '1' + bytes do bloco
Se não for: '0' + esq + dir
*/
fn push_tree_into_bitvec<const COUNT: usize>(tree: &mut Node<COUNT>, bitvec: &mut bv::BitVec) {
    //se alcançou um valor
    if let Some(valor) = tree.valor {
        bitvec.push(true);
        push_byte_array_into_bitvec(&valor, bitvec);
    }
    //se não alcançou
    else {
        bitvec.push(false);
        push_tree_into_bitvec(tree.esq.as_mut().unwrap(), bitvec);
        push_tree_into_bitvec(tree.dir.as_mut().unwrap(), bitvec);
    }
}

//converte BitVec para Vec<bool>. Se não tiver múltiplo de 8, completa com 0 os LSBs
fn bitvec_to_bytevec(bitvec: &bv::BitVec) -> Vec<u8> {
    let mut bytevec = Vec::with_capacity(bitvec.len() / 8 + 1);
    let mut byte_construido: u8 = 0;
    let mut bits_ocupados: u8 = 0;
    for b in bitvec {
        //adiciona o bit no byte
        byte_construido |= (if *b { 1 } else { 0 }) << 7 - bits_ocupados;
        bits_ocupados += 1;
        //se o byte estiver completo, manda para o vetor
        if bits_ocupados == 8 {
            bytevec.push(byte_construido);
            bits_ocupados = 0;
            byte_construido = 0;
        }
    }
    if bits_ocupados > 0 {
        //adiciona o que sobrou
        bytevec.push(byte_construido);
    }
    bytevec
}

//lê um u32 da posição e quantidade de bits do BitVec
fn get_u32_from_bitvec(bitvec: &bv::BitVec, inicio: &mut usize, tamanho: u8) -> u32 {
    let mut num = 0;
    let top = *inicio + tamanho as usize;
    while *inicio < top {
        num <<= 1;
        num |= if bitvec[*inicio] { 1 } else { 0 };
        *inicio += 1;
    }
    num
}

//lê um array de bytes a partir da posição especificada do BitVec
fn get_bytes_from_bitvec<const COUNT: usize>(
    bitvec: &bv::BitVec,
    inicio: &mut usize,
) -> [u8; COUNT] {
    let mut saida = [0; COUNT];
    for i in 0..COUNT {
        saida[i] = get_u32_from_bitvec(bitvec, inicio, 8) as u8;
    }
    saida
}

//recursivamente lê a árvore de huffman do BitVec, segundo as regras de escrita
//(ver em push_tree_into_bitvec)
fn get_tree_from_bitvec<const COUNT: usize>(
    bitvec: &bv::BitVec,
    inicio: &mut usize,
) -> Node<COUNT> {
    //se o bit do node indica que "é folha"
    if bitvec[*inicio] {
        *inicio += 1;
        return Node::new(Some(get_bytes_from_bitvec(bitvec, inicio)));
    }
    //se não é folha
    *inicio += 1;
    let mut node = Node::new(None);
    node.esq = Some(Box::new(get_tree_from_bitvec(bitvec, inicio)));
    node.dir = Some(Box::new(get_tree_from_bitvec(bitvec, inicio)));
    node
}
