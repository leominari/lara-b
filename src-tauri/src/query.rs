use crate::db::Message;

pub fn build_prompt(messages: &[Message], question: &str) -> String {
    // messages arrive newest-first (ORDER BY timestamp DESC) — reverse for prompt
    let mut ordered: Vec<&Message> = messages.iter().collect();
    ordered.reverse(); // oldest first in prompt

    let mut lines = String::new();
    for msg in &ordered {
        // Use the free function (not associated method) — from_timestamp(i64, u32) -> Option<DateTime<Utc>>
        let dt = chrono::DateTime::from_timestamp(msg.timestamp, 0)
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| msg.timestamp.to_string());
        let label = if msg.is_mine {
            format!("{} (você)", msg.contact)
        } else {
            msg.contact.clone()
        };
        lines.push_str(&format!("[{}] {}: {}\n", dt, label, msg.body));
    }

    let header = if lines.is_empty() {
        "Nenhuma mensagem encontrada no período solicitado.\n".to_string()
    } else {
        format!("Mensagens recentes (mais novas por último):\n\n{}", lines)
    };

    format!(
        "Você é um assistente pessoal super objetivo. Analise as mensagens abaixo e responda à pergunta do usuário de forma direta e concisa.\n\nIMPORTANTE — formato de exibição: sua resposta aparece em um widget flutuante com balões de 3 linhas (~120 caracteres cada). Estruture a resposta em parágrafos curtos de no máximo 2 frases, separados por linha em branco. Cada parágrafo deve ser uma ideia completa e autossuficiente. Nunca termine um parágrafo no meio de uma frase. Pode usar markdown simples (negrito, itálico, listas curtas).\n\n{}\nPergunta: {}",
        header, question
    )
}

pub fn build_contact_summary_prompt(messages: &[Message], contact: &str) -> String {
    let mut ordered: Vec<&Message> = messages.iter().collect();
    ordered.reverse();

    let mut lines = String::new();
    for msg in &ordered {
        let dt = chrono::DateTime::from_timestamp(msg.timestamp, 0)
            .map(|d| d.format("%H:%M").to_string())
            .unwrap_or_else(|| msg.timestamp.to_string());
        lines.push_str(&format!("[{}] {}\n", dt, msg.body));
    }

    format!(
        "Você é um assistente pessoal. Resuma em 2-3 frases curtas o que {} está falando nas mensagens abaixo. Seja direto. Use parágrafos curtos separados por linha em branco. Cada parágrafo deve ter no máximo 2 frases.\n\nMensagens de {}:\n{}",
        contact, contact, lines
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(id: &str, contact: &str, body: &str, ts: i64, is_mine: bool) -> Message {
        Message { id: id.into(), contact: contact.into(), chat: contact.into(), body: body.into(), timestamp: ts, is_mine }
    }

    #[test]
    fn test_oldest_message_first_in_prompt() {
        // Simulate DESC order from SQLite: newest first in slice
        let messages = vec![msg("2", "João", "segundo", 2000, false), msg("1", "João", "primeiro", 1000, false)];
        let prompt = build_prompt(&messages, "test");
        let pos1 = prompt.find("primeiro").unwrap();
        let pos2 = prompt.find("segundo").unwrap();
        assert!(pos1 < pos2, "oldest message must appear before newest in prompt");
    }

    #[test]
    fn test_sent_messages_labeled_voce() {
        let messages = vec![msg("1", "Me", "oi", 1000, true)];
        let prompt = build_prompt(&messages, "test");
        assert!(prompt.contains("(você)"), "sent messages must be labeled (você)");
    }

    #[test]
    fn test_received_messages_not_labeled_voce() {
        let messages = vec![msg("1", "João", "oi", 1000, false)];
        let prompt = build_prompt(&messages, "test");
        assert!(!prompt.contains("(você)"), "received messages must not have (você) label");
    }

    #[test]
    fn test_empty_messages_includes_fallback_text() {
        let prompt = build_prompt(&[], "test");
        assert!(prompt.contains("Nenhuma mensagem"));
    }

    #[test]
    fn test_question_included_in_prompt() {
        let prompt = build_prompt(&[], "tem algo urgente?");
        assert!(prompt.contains("tem algo urgente?"));
    }
}
