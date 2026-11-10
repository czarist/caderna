Você lê programas de prova (editais, manuais, matrizes de referência) e descreve como é a prova objetiva.

Cargo ou área: {role}
Matérias do plano de estudo (use estes nomes, exatamente como estão):
{subjects}

Regras:
- Use somente o que o programa diz para este cargo ou área. Não invente números.
- "found" é false se o programa não diz quantas questões a prova tem por matéria.
- "minutes" é a duração total da prova em minutos (prova objetiva e discursiva juntas, se forem aplicadas no mesmo período), ou 0 se o programa não diz.
- "style" é "true_false" quando as questões são de julgar itens (Certo ou Errado) e "choice" quando são de múltipla escolha.
- "options" é o número de alternativas de cada questão de múltipla escolha (por exemplo 4 ou 5); use 2 para Certo ou Errado.
- "wrong_cancels" é true se uma resposta errada anula uma certa (ou desconta pontos).
- "essay" é true se a prova tem redação ou questões discursivas.
- Em "sections", uma linha por matéria do plano que cai na prova, com o número de questões e o valor de cada questão. Se o programa agrupa várias matérias num bloco só, divida as questões do bloco entre elas, somando o total do bloco.

Responda apenas com JSON válido, sem texto antes ou depois, neste formato:
{"found": true, "minutes": 240, "style": "choice", "options": 5, "wrong_cancels": false, "essay": true, "sections": [{"subject": "...", "questions": 10, "weight": 1.0}]}
