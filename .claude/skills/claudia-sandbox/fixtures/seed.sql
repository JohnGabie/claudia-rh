INSERT INTO vagas (titulo, empresa, plataforma, url, localizacao, modelo_trabalho, idioma, descoberta_em, status, match_score) VALUES
 ('Backend Developer', 'Exemplo Ltda', 'linkedin', 'https://example.com/vagas/1', 'Remoto', 'remoto', 'pt', '2026-01-10T09:00:00Z', 'descoberta', 'alto'),
 ('Fullstack Engineer', 'Demo SA', 'linkedin', 'https://example.com/vagas/2', 'Lisboa', 'hibrido', 'en', '2026-01-11T09:00:00Z', 'candidatada', 'medio'),
 ('Data Analyst', 'Teste Corp', 'outra', 'https://example.com/vagas/3', 'Porto', 'presencial', 'pt', '2026-01-12T09:00:00Z', 'descartada', 'baixo');
INSERT INTO candidaturas (vaga_id, enviada_em, pasta_arquivos, metodo) VALUES
 (2, '2026-01-11T10:00:00Z', 'candidaturas/2', 'formulario');
INSERT INTO pendencias (vaga_id, criada_em, categoria, descricao) VALUES
 (1, '2026-01-10T09:30:00Z', 'pergunta', 'Pretensao salarial nao definida no perfil');
INSERT INTO propostas_perfil (vaga_id, criada_em, pergunta, contexto) VALUES
 (1, '2026-01-10T09:31:00Z', 'Tem experiencia com Kubernetes?', 'Requisito da vaga 1');
INSERT INTO sessoes (iniciada_em, terminada_em, motivo_disparo, motivo_termino, vagas_processadas) VALUES
 ('2026-01-11T09:00:00Z', '2026-01-11T09:40:00Z', 'manual', 'concluida', 3);
