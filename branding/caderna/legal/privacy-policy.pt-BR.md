# Política de Privacidade do {{app}}

Rascunho para revisão do cliente. Prestadora do serviço aos alunos: {{provider}}. Licenciadora do software: {{licensor}}. Contato da prestadora: {{contact}}. Este texto deve ser revisado com o contrato entre a prestadora e a licenciadora antes da distribuição.

Versão de 11 de novembro de 2026 ({{version}}), de acordo com a Lei Geral de Proteção de
Dados (Lei nº 13.709/2018, LGPD). O {{app}} é oferecido apenas no Brasil.

## 1. Resumo

- O {{app}} roda no seu computador. Seus estudos (editais, planos, progresso, anotações,
  fotos e respostas) ficam nele.
- Se a sua escola cadastrou você como **aluno ou professor** de uma turma, o painel da
  {{company}} também guarda: os planos que os professores publicam para as turmas e, para
  cada aluno, o **progresso nesses planos** (tópicos, quizzes, aulas, revisões, caderno de
  erros e tempo de estudo), para que ele volte num outro computador e para que os
  professores da escola acompanhem a turma.
- Para economizar IA, o {{app}} consulta uma **base comum** de materiais que a IA já fez
  (planos, apostilas e questões) antes de pedir à IA, e compartilha nela o que a IA fizer.
  Essa base nunca recebe dados de alunos, mensagens, respostas nem o material de provas
  anteriores que a escola enviou.
- O {{app}} se conecta ao **painel da {{company}}** para confirmar a licença deste
  computador e para usar a inteligência artificial (IA) incluída na licença.
- Para isso, o painel recebe a chave de licença, um código deste computador gerado por
  hash, a versão do app, o sistema operacional e o conteúdo de cada tarefa de IA. O painel
  **não guarda o conteúdo** das tarefas de IA: ele o repassa ao fornecedor de IA contratado
  pela {{company}} e registra só a quantidade de uso.
- Não há telemetria, estatísticas de uso do app, anúncios nem venda de dados.

## 2. Quem é responsável

- **{{provider}}** oferece o {{app}} aos alunos e opera o painel. É a controladora dos
  dados tratados pelo painel (licença, computadores ativados e uso de IA) e quem atende os
  seus pedidos sobre esses dados. Contato: {{contact}}.
- **{{licensor}}** desenvolve o software e o licencia à prestadora. O aplicativo não envia
  dados de estudo à licenciadora.
- **O fornecedor de IA contratado pela prestadora** recebe o conteúdo de cada tarefa de IA
  por meio do painel e trata esse conteúdo conforme o contrato com a prestadora.

## 3. O que fica no seu computador

Ficam na pasta de dados do aplicativo (`%LOCALAPPDATA%\{{data_dir}}` no Windows, ou na pasta
definida em `KG_DATA_DIR`):

- seu nome (como o tutor chama você), idioma, professor e demais preferências;
- a chave de licença, sem criptografia no banco de dados local, e a data do último contato
  com o painel;
- o token do computador, que o painel renova a cada 7 dias, guardado no cofre de
  credenciais do sistema operacional (Gerenciador de Credenciais no Windows, Chaveiro no
  macOS, chaveiro do kernel no Linux). Se o cofre não estiver disponível, o token fica só na
  memória enquanto o app está aberto;
- a identidade visual da {{company}} recebida do painel: nome, cores, links, imagens (ícone,
  capa, ilustração, marca e marca-d'água) e textos de telas do app;
- editais, provas, gabaritos e materiais enviados, além dos planos, do progresso, dos
  quizzes, das revisões, das apostilas, dos destaques e das notas;
- conversas das aulas e o caderno de erros;
- fotos das respostas discursivas, suas transcrições e avaliações;
- arquivos do modelo PaddleOCR-VL, baixados para a pasta local de modelos se você instalar
  o OCR local;
- se a sua escola bloquear capturas de tela para os alunos: o seu nome e o seu email,
  recebidos do painel, que aparecem em marca-d'água clara nas páginas das apostilas para
  identificar de quem é a cópia; deixam de ser usados quando a escola libera as capturas;
- a data e a hora do seu aceite destes documentos;
- um arquivo de registro técnico (log), que não guarda imagens, respostas nem chaves.

Com as capturas de tela bloqueadas pela escola, no macOS e no Linux o app confere cerca de
uma vez por segundo os nomes dos programas abertos no computador, para encobrir o conteúdo
enquanto um programa de gravação de tela estiver aberto. Essa lista é lida no próprio
computador e não é guardada nem enviada a ninguém. No Windows, o próprio sistema deixa a
janela fora das capturas.

O áudio do microfone é transcrito no próprio computador e não é enviado a ninguém. As
gravações temporárias de diagnóstico ficam só na pasta de dados.

## 4. O que sai do seu computador, e para quem

| Para onde | O quê | Quando |
|---|---|---|
| Painel da {{company}}, ou o endereço que ele indicar para as imagens da identidade visual | Pedidos de download das imagens, que mostram o seu endereço IP (sem a chave nem o token) | Quando a identidade visual muda no painel |
| Painel da {{company}} | A chave de licença; um código deste computador (o identificador do sistema operacional passado por hash, que não revela o identificador original); a versão do app; o sistema operacional (Windows, macOS ou Linux); o endereço IP | Ao ativar a licença, ao abrir o app e a cada hora com o app aberto |
| Painel da {{company}}, que repassa ao fornecedor de IA contratado por ela | Trechos do programa da prova e dos materiais, tópicos, suas mensagens e respostas transcritas, seu nome (para o tutor chamar você), textos das apostilas | Em cada tarefa de IA |
| PaddleOCR-VL rodando localmente pelo `llama-server` v0.8.0 do llama.cpp | Fotos das respostas discursivas | Por padrão; processadas no computador, não enviadas ao painel |
| Painel da {{company}}, para o modelo de visão do fornecedor de IA | Fotos das respostas discursivas e páginas de PDFs escaneados | Fotos de respostas só se você escolher explicitamente o OCR externo; páginas de PDFs escaneados só quando você pedir a leitura |
| Wikipédia e Wikimedia Commons | O termo de busca de uma imagem do quadro ou da apostila e, para dar o crédito, o nome do arquivo de imagem que a própria Wikipédia indicou | Se a opção de imagens estiver ligada |
| Hugging Face, GitHub e sites das vozes | Pedidos de download, que mostram o seu endereço IP | Quando você baixa modelos de voz, fala ou OCR |
| GitHub | Um pedido da versão mais recente, que mostra o seu endereço IP e a versão do app; o pacote da nova versão, se você escolher atualizar | Cada vez que o app abre, se a busca de atualizações estiver ligada |
| Serviço de voz na nuvem | O texto que o tutor vai falar | Só se você ativar a voz na nuvem |

O nome do computador e o seu nome **não** são enviados ao painel para ativar a licença.

**Turmas (só para quem a escola cadastrou como aluno ou professor):**

| Para | O que | Quando |
|---|---|---|
| Painel da {{company}} | Do professor: os planos que ele publica (título, prova, finalidade, matérias, tópicos, o programa da prova, apostilas e questões) e as turmas escolhidas | Quando o professor publica |
| Painel da {{company}} | Do aluno: o progresso nos planos das turmas (situação e agenda de revisão de cada tópico, resultados dos quizzes, resumos das aulas, revisões feitas, questões do caderno de erros) e o tempo de estudo por dia | A cada hora com o app aberto e em "Sincronizar agora" |
| Painel da {{company}}, base comum | Planos, apostilas e questões que a IA fez, com a matéria, o tópico, o idioma e a finalidade; uma impressão digital (hash) do pedido | Antes e depois de cada plano, apostila ou banco de questões feito pela IA |

Os professores da escola veem, para cada aluno das turmas deles, o nome e um resumo do
progresso: tópicos dominados, acertos nos quizzes, aulas, tempo de estudo e a última
atividade. A escola organiza as turmas e as pessoas na área do cliente do painel. A base
comum é usada por todas as escolas; ela não guarda quem pediu cada material, só a escola
que o gerou primeiro, visível apenas para a {{company}}.

O painel guarda, para cada computador ativado: o código por hash, o sistema operacional, a
versão do app e as datas de ativação e do último contato. Para cada tarefa de IA, guarda
apenas o modelo usado, a quantidade de texto processado (tokens), o resultado e a duração,
para controlar a cota da licença. O conteúdo das tarefas não fica guardado no painel,
exceto o que vai para a base comum e os planos publicados para as turmas, descritos acima.

Os pacotes para Windows e Linux incluem um runtime do llama.cpp com Vulkan. Quando há uma
GPU compatível, o app usa o driver Vulkan já instalado no computador; se não encontrar
dispositivo ou a inicialização da GPU falhar, o OCR volta para CPU. No macOS, usa CPU. O
runtime e o driver processam o OCR localmente: o backend Vulkan não é um destinatário e
não transmite as fotos ao painel, à {{licensor}} ou a fornecedores.

O fornecedor de IA contratado pela {{company}} pode ficar fora do Brasil. Nesse caso, o
conteúdo das tarefas de IA é transferido para outro país (art. 33 da LGPD), com as
garantias previstas no contrato entre a prestadora e o fornecedor.

## 5. Finalidade e base legal

- **Licença:** confirmar que este computador pode usar a licença, contar as vagas e
  permitir desconectar ou bloquear computadores. Base legal: execução do contrato (art. 7º,
  V, da LGPD).
- **IA:** montar o plano, dar aulas, gerar questões e apostilas e avaliar respostas, e
  controlar a cota de uso da licença. Base legal: execução do contrato.
- **No seu computador:** mostrar o seu progresso e guardar os seus estudos.
- **Turmas:** entregar aos alunos os planos dos professores, guardar o progresso de cada
  aluno para outro computador e mostrar aos professores como a turma está indo. Base legal:
  execução do contrato entre você (ou a sua escola) e a {{company}}.
- **Restrições da escola:** quando a escola bloqueia a exportação das apostilas ou as
  capturas de tela para os alunos, aplicar esse bloqueio e marcar as apostilas com o nome e
  o email do aluno, para proteger o material da escola e identificar cópias vazadas. Base
  legal: execução do contrato entre você (ou a sua escola) e a {{company}}, e o legítimo
  interesse da escola em proteger o seu material (art. 7º, V e IX, da LGPD).
- **Base comum:** reaproveitar materiais já feitos pela IA, gastando menos IA e entregando
  mais rápido. Não envolve dados pessoais.

## 6. Sem conexão

O {{app}} confirma a licença com o painel ao abrir e a cada hora. Sem internet, ele
continua funcionando por até 14 dias desde o último contato; depois disso, pede uma conexão
para continuar. As tarefas de IA precisam de internet. Seus estudos ficam guardados no
computador em qualquer caso.

## 7. Segurança

Os dados ficam no seu computador, protegidos pelo acesso à sua conta de usuário. Proteja o
computador com senha. A conexão com o painel usa HTTPS. O token do computador fica no cofre
de credenciais do sistema operacional e vale por 7 dias. A chave de licença fica sem
criptografia no banco local. Problemas de segurança podem ser relatados conforme o
arquivo SECURITY.md do projeto.

## 8. Por quanto tempo

- **No seu computador:** os dados ficam até você apagá-los. Excluir uma tentativa
  discursiva apaga as fotos dela. Apagar a pasta de dados remove tudo.
- **No painel:** os registros do computador ficam enquanto a licença existir; os registros
  de uso de IA, pelo prazo definido pela prestadora para controlar as cotas. O progresso de
  um aluno fica enquanto ele estiver cadastrado pela escola; quando a escola remove o aluno
  ou o curso, o progresso nele é apagado. Os materiais da base comum ficam até a
  {{company}} removê-los.
- **"Desconectar este computador"** (em Configurações) libera a vaga da licença e apaga a
  chave e o token deste computador. Os seus estudos ficam.

## 9. Encarregado pelo tratamento de dados

O encarregado pelo tratamento de dados pessoais da {{company}} (art. 41 da LGPD) recebe
reclamações, pedidos e comunicações dos titulares e da ANPD pelo contato {{contact}}.

## 10. Seus direitos

A LGPD garante, entre outros, os direitos de acesso, correção, eliminação, portabilidade e
informação sobre compartilhamento (art. 18).

- **No aplicativo:** você vê, corrige e apaga os seus dados diretamente; o progresso pode
  ser exportado.
- **No painel:** peça à {{company}} pelo contato {{contact}}.
- **Reclamação:** você também pode recorrer à Autoridade Nacional de Proteção de Dados
  (ANPD).

## 11. Crianças e adolescentes

Menores de 18 anos devem usar o aplicativo com o conhecimento e o consentimento dos pais ou
responsáveis (art. 14 da LGPD).

## 12. Inteligência artificial

As respostas geradas pela IA podem conter erros, e a qualidade depende dos limites do
modelo usado pela {{company}}. Veja os Termos de Uso, item 6.

## 13. Alterações

Quando esta Política mudar, a nova versão terá uma nova data, e o aplicativo pedirá um novo
aceite.

## 14. Contato

Dúvidas sobre privacidade: {{contact}}.
