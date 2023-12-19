use std::cmp::max;
#[derive(Clone, Debug)]
pub struct Diff {
    pub lineas_eliminadas: Vec<(usize, String)>,
    pub lineas_agregadas: Vec<(usize, String)>,
    pub lineas: Vec<(usize, bool, String)>,
    pub lineas_extra: usize,
}

#[derive(Clone, Debug)]
struct Celda {
    valor: usize,
    es_match: bool,
    fila: usize,
    columna: usize,
}

fn empty_diff() -> Diff {
    Diff {
        lineas_eliminadas: vec![],
        lineas_agregadas: vec![],
        lineas: vec![],
        lineas_extra: 0,
    }
}

fn valor_match(matriz: &[Vec<Celda>], i: usize, j: usize) -> usize {
    if i == 0 || j == 0 || (i, j) == (0, 0) {
        1
    } else {
        matriz[i - 1][j - 1].valor + 1
    }
}

fn valor_unmatch(matriz: &[Vec<Celda>], i: usize, j: usize) -> usize {
    if i == 0 && j == 0 {
        0
    } else if i == 0 {
        return matriz[i][j - 1].valor;
    } else if j == 0 {
        return matriz[i - 1][j].valor;
    } else {
        return max(matriz[i - 1][j].valor, matriz[i][j - 1].valor);
    }
}

fn get_diff(
    matriz_coincidencias: Vec<Vec<Celda>>,
    len_columna: usize,
    len_fila: usize,
) -> (Vec<usize>, Vec<usize>) {
    let mut stack = Vec::new();

    let mut j = len_columna;
    let mut i = len_fila;

    let mut num_bloque_actual = matriz_coincidencias[j][i].valor;
    loop {
        if i == 0 && j == 0 {
            if matriz_coincidencias[j][i].es_match {
                stack.push(matriz_coincidencias[j][i].clone());
            }
            break;
        }

        if matriz_coincidencias[j][i].es_match {
            stack.push(matriz_coincidencias[j][i].clone());
            num_bloque_actual -= 1;
            i = i.saturating_sub(1);
            j = j.saturating_sub(1);
        } else {
            if j == 0 && matriz_coincidencias[j][i - 1].valor == num_bloque_actual {
                i = i.saturating_sub(1);
                j = j.saturating_sub(1);
                continue;
            }

            if i == 0 && matriz_coincidencias[j - 1][i].valor == num_bloque_actual {
                i = i.saturating_sub(1);
                j = j.saturating_sub(1);
                continue;
            }

            if matriz_coincidencias[j - 1][i - 1].valor == num_bloque_actual {
                i = i.saturating_sub(1);
                j = j.saturating_sub(1);
                continue;
            } else {
                let mut k = j;
                let mut la_encontre_yendo_arriba = false;

                loop {
                    if matriz_coincidencias[k][i].es_match {
                        la_encontre_yendo_arriba = true;
                        j = k;

                        stack.push(matriz_coincidencias[j][i].clone());
                        i = i.saturating_sub(1);
                        j = j.saturating_sub(1);

                        num_bloque_actual -= 1;

                        break;
                    }

                    if matriz_coincidencias[k - 1][i].valor != num_bloque_actual {
                        break;
                    }

                    k = k.saturating_sub(1);
                }
                if !la_encontre_yendo_arriba {
                    let mut k = i;

                    loop {
                        if matriz_coincidencias[j][k].es_match {
                            i = k;
                            stack.push(matriz_coincidencias[j][i].clone());
                            i = i.saturating_sub(1);
                            j = j.saturating_sub(1);

                            num_bloque_actual -= 1;

                            break;
                        }

                        if matriz_coincidencias[j][k - 1].valor != num_bloque_actual {
                            break;
                        }
                        k = k.saturating_sub(1);
                    }
                }
            }
        }
    }
    let base_numbers = stack.iter().map(|x| x.fila).collect::<Vec<usize>>();
    let new_numbers = stack.iter().map(|x| x.columna).collect::<Vec<usize>>();

    let mut lineas_eliminadas = Vec::new();
    for i in 0..(len_columna + 1) {
        if !base_numbers.contains(&i) {
            lineas_eliminadas.push(i);
        }
    }

    let mut lineas_agregadas = Vec::new();
    for i in 0..(len_fila + 1) {
        if !new_numbers.contains(&i) {
            lineas_agregadas.push(i);
        }
    }

    (lineas_eliminadas, lineas_agregadas)
}

impl Diff {
    pub fn new(base: String, new: String) -> Diff {
        if base == new {
            return empty_diff();
        }
        if base.is_empty() {
            let mut only_add_diff = Diff {
                lineas_eliminadas: vec![],
                lineas_agregadas: vec![],
                lineas: vec![],
                lineas_extra: 0,
            };
            let new_lines = new.split('\n').collect::<Vec<&str>>();
            for (i, line) in new_lines.iter().enumerate() {
                only_add_diff.lineas.push((i, true, line.to_string()));
            }
            return only_add_diff;
        }
        let base_lines = base.lines().collect::<Vec<&str>>();
        let new_lines = new.lines().collect::<Vec<&str>>();

        let mut matriz_coincidencias: Vec<Vec<Celda>> = vec![vec![]];

        for (i, base_line) in base_lines.iter().enumerate() {
            for (j, new_line) in new_lines.iter().enumerate() {
                if base_line == new_line {
                    let next_value = valor_match(&matriz_coincidencias, i, j);
                    matriz_coincidencias[i].push(Celda {
                        valor: next_value,
                        es_match: true,
                        //valor_matcheado: base_line.to_string(),
                        fila: i,
                        columna: j,
                    });
                } else {
                    let next_value = valor_unmatch(&matriz_coincidencias, i, j);
                    matriz_coincidencias[i].push(Celda {
                        valor: next_value,
                        es_match: false,
                        fila: i,
                        columna: j,
                    });
                }
            }

            matriz_coincidencias.push(vec![]);
        }

        let (indices_lineas_eliminadas, indices_lineas_agregadas) = get_diff(
            matriz_coincidencias,
            base_lines.len() - 1,
            new_lines.len() - 1,
        );

        let mut lineas_eliminadas = Vec::new();
        let mut lineas_agregadas = Vec::new();

        let mut lineas = Vec::new();

        for (i, line) in base_lines.iter().enumerate() {
            if indices_lineas_eliminadas.contains(&i) {
                lineas.push((i, false, line.to_string()));
            }
        }
        for (i, line) in new_lines.iter().enumerate() {
            if indices_lineas_agregadas.contains(&i) {
                lineas.push((i, true, line.to_string()));
            }
        }
        lineas.sort_by(|a, b| a.0.cmp(&b.0)); //ordeno ascendente

        for (i, line) in base_lines.iter().enumerate() {
            if indices_lineas_eliminadas.contains(&i) {
                lineas_eliminadas.push((i, line.to_string()));
            }
        }
        for (i, line) in new_lines.iter().enumerate() {
            if indices_lineas_agregadas.contains(&i) {
                lineas_agregadas.push((i, line.to_string()));
            }
        }

        Diff {
            lineas_eliminadas,
            lineas_agregadas,
            lineas,
            lineas_extra: 0,
        }
    }

    pub fn has_delete_diff(&self, i: usize) -> bool {
        for line in self.lineas_eliminadas.iter() {
            if line.0 == i {
                return true;
            }
        }
        false
    }

    pub fn has_add_diff(&self, i: usize) -> (bool, String) {
        let linea: (bool, String) = (false, "".to_string());
        for line in self.lineas_agregadas.iter() {
            if line.0 == i {
                return (true, line.1.clone());
            }
        }
        linea
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test00_diff_entre_string_iguales_esta_vacio() {
        let base = "hola".to_string();
        let new = "hola".to_string();
        let diff = Diff::new(base, new);
        assert!(diff.lineas_eliminadas.is_empty());
    }
}
