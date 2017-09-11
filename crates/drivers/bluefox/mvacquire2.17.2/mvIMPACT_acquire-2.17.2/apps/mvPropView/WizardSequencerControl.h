//-----------------------------------------------------------------------------
#ifndef WizardSequencerControlH
#define WizardSequencerControlH WizardSequencerControlH
//-----------------------------------------------------------------------------
#include <mvIMPACT_CPP/mvIMPACT_acquire.h>
#include <mvIMPACT_CPP/mvIMPACT_acquire_GenICam.h>
#include "ValuesFromUserDlg.h"

class wxCheckBox;
class wxComboBox;
class wxSlider;
class wxSpinCtrl;

//-----------------------------------------------------------------------------
class WizardSequencerControl : public OkAndCancelDlg
//-----------------------------------------------------------------------------
{
public:
    explicit WizardSequencerControl( wxWindow* pParent, const wxString& title, mvIMPACT::acquire::Device* pDev, size_t displayCount, const std::vector<long>& setToDisplayTable );
    virtual ~WizardSequencerControl();

    const std::vector<int64_type>& GetSequencerSetNextTable( void ) const;
    const std::vector<long>& GetSetToDisplayTable( void ) const;
private:
    //-----------------------------------------------------------------------------
    enum TWidgetIDs_SequencerControl
    //-----------------------------------------------------------------------------
    {
        widMainFrame = widFirst
    };
    //-----------------------------------------------------------------------------
    /// \brief GUI elements for a single sequencer set
    struct SequencerSetControls
            //-----------------------------------------------------------------------------
    {
        wxSpinCtrl* pSequencerSetNext_;
        std::vector<wxStaticText*> sequenceableFeatures_;
        wxComboBox* pDisplayToUse_;
    };
    std::vector<HOBJ> sequenceableFeatures_;
    std::vector<SequencerSetControls*> sequencerSetGUIData_;
    mutable std::vector<int64_type> sequencerSetNextTable_;
    mutable std::vector<long> setToDisplayTable_;
    wxArrayString displayCoices_;
    bool boGUICreated_;
    mvIMPACT::acquire::GenICam::SequencerControl sc_;

    void CreateSequencerSetControls( int64_type index, wxPanel* pPanel, wxFlexGridSizer* pSequencerControlControlsSizer );
};

#endif // WizardSequencerControlH
