// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

use std::collections::HashSet;

pub(super) use anki_proto::notes::notes_service::Service as NotesService;

use super::Backend;
use crate::cloze::add_cloze_numbers_in_string;
use crate::prelude::*;

impl NotesService for Backend {
    type Error = AnkiError;

    fn new_note(
        &self,
        input: anki_proto::notetypes::NotetypeId,
    ) -> Result<anki_proto::notes::Note> {
        let ntid = input.into();
        self.with_col(|col| {
            let nt = col.get_notetype(ntid)?.or_not_found(ntid)?;
            Ok(nt.new_note().into())
        })
    }

    fn add_note(
        &self,
        input: anki_proto::notes::AddNoteRequest,
    ) -> Result<anki_proto::notes::AddNoteResponse> {
        self.with_col(|col| {
            let mut note: Note = input.note.or_invalid("no note provided")?.into();
            let changes = col.add_note(&mut note, DeckId(input.deck_id))?;
            Ok(anki_proto::notes::AddNoteResponse {
                note_id: note.id.0,
                changes: Some(changes.into()),
            })
        })
    }

    fn defaults_for_adding(
        &self,
        input: anki_proto::notes::DefaultsForAddingRequest,
    ) -> Result<anki_proto::notes::DeckAndNotetype> {
        self.with_col(|col| {
            let home_deck: DeckId = input.home_deck_of_current_review_card.into();
            col.defaults_for_adding(home_deck).map(Into::into)
        })
    }

    fn default_deck_for_notetype(
        &self,
        input: anki_proto::notetypes::NotetypeId,
    ) -> Result<anki_proto::decks::DeckId> {
        self.with_col(|col| {
            Ok(col
                .default_deck_for_notetype(input.into())?
                .unwrap_or(DeckId(0))
                .into())
        })
    }

    fn update_notes(
        &self,
        input: anki_proto::notes::UpdateNotesRequest,
    ) -> Result<anki_proto::collection::OpChanges> {
        self.with_col(|col| {
            let notes = input
                .notes
                .into_iter()
                .map(Into::into)
                .collect::<Vec<Note>>();
            col.update_notes_maybe_undoable(notes, !input.skip_undo_entry)
        })
        .map(Into::into)
    }

    fn get_note(&self, input: anki_proto::notes::NoteId) -> Result<anki_proto::notes::Note> {
        let nid = input.into();
        self.with_col(|col| col.storage.get_note(nid)?.or_not_found(nid).map(Into::into))
    }

    fn remove_notes(
        &self,
        input: anki_proto::notes::RemoveNotesRequest,
    ) -> Result<anki_proto::collection::OpChangesWithCount> {
        self.with_col(|col| {
            if !input.note_ids.is_empty() {
                col.remove_notes(
                    &input
                        .note_ids
                        .into_iter()
                        .map(Into::into)
                        .collect::<Vec<_>>(),
                )
            } else {
                let nids = col.storage.note_ids_of_cards(
                    &input
                        .card_ids
                        .into_iter()
                        .map(Into::into)
                        .collect::<Vec<_>>(),
                )?;
                col.remove_notes(&nids.into_iter().collect::<Vec<_>>())
            }
            .map(Into::into)
        })
    }

    fn cloze_numbers_in_note(
        &self,
        note: anki_proto::notes::Note,
    ) -> Result<anki_proto::notes::ClozeNumbersInNoteResponse> {
        let mut set = HashSet::with_capacity(4);
        for field in &note.fields {
            add_cloze_numbers_in_string(field, &mut set);
        }
        Ok(anki_proto::notes::ClozeNumbersInNoteResponse {
            numbers: set.into_iter().map(|n| n as u32).collect(),
        })
    }

    fn after_note_updates(
        &self,
        input: anki_proto::notes::AfterNoteUpdatesRequest,
    ) -> Result<anki_proto::collection::OpChangesWithCount> {
        self.with_col(|col| {
            col.after_note_updates(
                &to_note_ids(input.nids),
                input.generate_cards,
                input.mark_notes_modified,
            )
            .map(Into::into)
        })
    }

    fn field_names_for_notes(
        &self,
        input: anki_proto::notes::FieldNamesForNotesRequest,
    ) -> Result<anki_proto::notes::FieldNamesForNotesResponse> {
        self.with_col(|col| {
            let nids: Vec<_> = input.nids.into_iter().map(NoteId).collect();
            col.storage
                .field_names_for_notes(&nids)
                .map(|fields| anki_proto::notes::FieldNamesForNotesResponse { fields })
        })
    }

    fn note_fields_check(
        &self,
        input: anki_proto::notes::Note,
    ) -> Result<anki_proto::notes::NoteFieldsCheckResponse> {
        let note: Note = input.into();
        self.with_col(|col| {
            col.note_fields_check(&note)
                .map(|r| anki_proto::notes::NoteFieldsCheckResponse { state: r as i32 })
        })
    }

    fn cards_of_note(
        &self,
        input: anki_proto::notes::NoteId,
    ) -> Result<anki_proto::cards::CardIds> {
        self.with_col(|col| {
            col.storage
                .all_card_ids_of_note_in_template_order(NoteId(input.nid))
                .map(|v| anki_proto::cards::CardIds {
                    cids: v.into_iter().map(Into::into).collect(),
                })
        })
    }

    fn get_single_notetype_of_notes(
        &self,
        input: anki_proto::notes::NoteIds,
    ) -> Result<anki_proto::notetypes::NotetypeId> {
        self.with_col(|col| {
            col.get_single_notetype_of_notes(&input.note_ids.into_newtype(NoteId))
                .map(Into::into)
        })
    }
}

pub(super) fn to_note_ids(ids: Vec<i64>) -> Vec<NoteId> {
    ids.into_iter().map(NoteId).collect()
}

pub(super) fn to_i64s(ids: Vec<NoteId>) -> Vec<i64> {
    ids.into_iter().map(Into::into).collect()
}

impl From<anki_proto::notes::NoteId> for NoteId {
    fn from(nid: anki_proto::notes::NoteId) -> Self {
        NoteId(nid.nid)
    }
}

impl From<NoteId> for anki_proto::notes::NoteId {
    fn from(nid: NoteId) -> Self {
        anki_proto::notes::NoteId { nid: nid.0 }
    }
}
