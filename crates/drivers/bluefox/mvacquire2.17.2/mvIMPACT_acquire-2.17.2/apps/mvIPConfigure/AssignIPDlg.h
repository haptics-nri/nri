//-----------------------------------------------------------------------------
#ifndef AssignIPDlgH
#define AssignIPDlgH AssignIPDlgH
//-----------------------------------------------------------------------------
#include "wx/wx.h"
#include "CustomValidators.h"
#include "TLILibImports.h"

class wxButton;
class wxComboBox;
class wxTextCtrl;
class IPConfigureFrame;

//-----------------------------------------------------------------------------
class AssignIPDlg : public wxDialog
//-----------------------------------------------------------------------------
{
    wxTextCtrl* pTCDeviceMACAddress_;
    wxTextCtrl* pTCDesiredIPAddress_;
    wxTextCtrl* pTCDesiredSubnetMask_;
    wxTextCtrl* pTCDesiredGateway_;
    wxButton* pBtnBuildMACAddress_;
    wxButton* pBtnExecute_;
    wxComboBox* pCBAdapterList_;
    IPConfigureFrame* pParent_;
    //-----------------------------------------------------------------------------
    enum TWidgetIDs
    //-----------------------------------------------------------------------------
    {
        widBtnExecute = 1,
        widTCMACAddress,
        widDesiredIPAddress,
        widBtnBuildMACAddress,
        widDesiredSubnetMask,
        widDesiredGateway,
        widAdapterList
    };
    MVTLI_HANDLE hTL_;
    IPv4StringValidator IPv4StringValidator_;
    MACStringValidator MACStringValidator_;
    void OnBtnExecute( wxCommandEvent& );
    void OnBtnBuildMACAddress( wxCommandEvent& );
    void OnDesiredIPAddressChanged( wxCommandEvent& );
public:
    AssignIPDlg( IPConfigureFrame* pParent, MVTLI_HANDLE hTL, const wxString& deviceMACAddress = wxEmptyString, const wxString& connectedIPAddress = wxEmptyString, bool boShowDifferentSubnetWarning = false, bool buildMACFromSerialEnabled = true );
    // any class wishing to process wxWidgets events must use this macro
    DECLARE_EVENT_TABLE()
};

bool ValidateIPDataSet( const wxString& IPAddress, const wxString& SubnetMask, const wxString& Gateway, wxWindow* pParent, IPConfigureFrame* pMainFrame );

#endif // AssignIPDlgH
